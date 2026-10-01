use rokbattles_capture_runtime::packet::{
    self, ClientTcpControl, FlowKey, SocketEstablishedEvidence, SocketRetiredEvidence,
};
use std::{
    fmt, io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAX_BODY_BYTES: usize = 65_535;
const HEADER_BYTES: usize = 12;
const MAGIC: &[u8; 4] = b"RKCI";
const VERSION: u8 = 1;
const CONTROL_BYTES: usize = 46;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientRequest {
    Start,
    Stop,
}

/// Native source actually selected by the helper, for local status only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Backend {
    WinDivert = 1,
    Pcap = 2,
}

/// Stable coarse reasons. Never send native errors, paths, tokens or captured data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum UnavailableReason {
    NativeBackend = 1,
    OwnershipUnavailable = 2,
    SessionEnded = 3,
    HelperNotProvisioned = 4,
    Busy = 5,
}

/// Owned packet bytes are scrubbed on all drop paths, including rejected peers,
/// queue overflow, parser failure and cancellation. Debug never contains bytes.
pub struct PacketBytes(zeroize::Zeroizing<Vec<u8>>);
impl PacketBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(zeroize::Zeroizing::new(bytes))
    }
}
impl From<Vec<u8>> for PacketBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::new(bytes)
    }
}
impl std::ops::Deref for PacketBytes {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl AsRef<[u8]> for PacketBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}
impl PartialEq for PacketBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}
impl Eq for PacketBytes {}
impl fmt::Debug for PacketBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PacketBytes(<redacted>)")
    }
}

#[derive(PartialEq, Eq)]
pub enum Record {
    Started(Backend),
    ServerPacket(PacketBytes),
    ClientControl(ClientTcpControl),
    SocketEstablished(SocketEstablishedEvidence),
    SocketRetired(SocketRetiredEvidence),
    /// All previously observed flow generations are invalid after a loss.
    Gap,
    Unavailable(UnavailableReason),
    Stopped,
    Keepalive,
}

impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Started(backend) => f.debug_tuple("Started").field(backend).finish(),
            Self::ServerPacket(_) => f.write_str("ServerPacket(<redacted>)"),
            Self::ClientControl(_) => f.write_str("ClientControl(<redacted>)"),
            Self::SocketEstablished(_) => f.write_str("SocketEstablished(<redacted>)"),
            Self::SocketRetired(_) => f.write_str("SocketRetired(<redacted>)"),
            Self::Gap => f.write_str("Gap"),
            Self::Unavailable(reason) => f.debug_tuple("Unavailable").field(reason).finish(),
            Self::Stopped => f.write_str("Stopped"),
            Self::Keepalive => f.write_str("Keepalive"),
        }
    }
}

/// One reader per authenticated transport. Keep its read future alive until a
/// complete frame arrives; cancellation is safe only when closing that transport.
/// Enforces startup ordering independently of any consumer/UI state.
pub struct SessionReader<R> {
    input: R,
    started: bool,
    ended: bool,
}
impl<R: AsyncRead + Unpin> SessionReader<R> {
    pub fn new(input: R) -> Self {
        Self { input, started: false, ended: false }
    }
    pub async fn read(&mut self) -> io::Result<Record> {
        if self.ended {
            return Err(invalid());
        }
        let record = match read_record(&mut self.input).await {
            Ok(record) => record,
            Err(error) => {
                self.ended = true;
                return Err(error);
            }
        };
        match &record {
            Record::Started(_) if !self.started => self.started = true,
            Record::Unavailable(_) => self.ended = true,
            Record::Started(_) => {
                self.ended = true;
                return Err(invalid());
            }
            _ if !self.started => {
                self.ended = true;
                return Err(invalid());
            }
            Record::Stopped => self.ended = true,
            _ => {}
        }
        Ok(record)
    }
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid local capture frame")
}

pub async fn read_request<R: AsyncRead + Unpin>(input: &mut R) -> io::Result<ClientRequest> {
    let (kind, body) = read_frame(input).await?;
    match (kind, body.as_ref()) {
        (1, []) => Ok(ClientRequest::Start),
        (2, []) => Ok(ClientRequest::Stop),
        _ => Err(invalid()),
    }
}

pub async fn write_request<W: AsyncWrite + Unpin>(
    output: &mut W,
    request: ClientRequest,
) -> io::Result<()> {
    write_frame(
        output,
        match request {
            ClientRequest::Start => 1,
            ClientRequest::Stop => 2,
        },
        &[],
    )
    .await
}

pub async fn read_record<R: AsyncRead + Unpin>(input: &mut R) -> io::Result<Record> {
    let (kind, body) = read_frame(input).await?;
    match (kind, body.as_ref()) {
        (16, [backend]) => Ok(Record::Started(match backend {
            1 => Backend::WinDivert,
            2 => Backend::Pcap,
            _ => return Err(invalid()),
        })),
        (17, _) if exact_server_packet(&body) => Ok(Record::ServerPacket(body)),
        (18, _) => decode_control(&body).map(Record::ClientControl),
        (19, []) => Ok(Record::Gap),
        (20, [reason]) => Ok(Record::Unavailable(match reason {
            1 => UnavailableReason::NativeBackend,
            2 => UnavailableReason::OwnershipUnavailable,
            3 => UnavailableReason::SessionEnded,
            4 => UnavailableReason::HelperNotProvisioned,
            5 => UnavailableReason::Busy,
            _ => return Err(invalid()),
        })),
        (21, []) => Ok(Record::Stopped),
        (22, []) => Ok(Record::Keepalive),
        (23, _) => decode_established(&body).map(Record::SocketEstablished),
        (24, _) => decode_retired(&body).map(Record::SocketRetired),
        _ => Err(invalid()),
    }
}

pub async fn write_record<W: AsyncWrite + Unpin>(
    output: &mut W,
    record: &Record,
) -> io::Result<()> {
    match record {
        Record::Started(backend) => write_frame(output, 16, &[*backend as u8]).await,
        Record::ServerPacket(body) if exact_server_packet(body) => {
            write_frame(output, 17, body).await
        }
        Record::ServerPacket(_) => Err(invalid()),
        Record::ClientControl(control) => write_frame(output, 18, &encode_control(*control)?).await,
        Record::Gap => write_frame(output, 19, &[]).await,
        Record::Unavailable(reason) => write_frame(output, 20, &[*reason as u8]).await,
        Record::Stopped => write_frame(output, 21, &[]).await,
        Record::Keepalive => write_frame(output, 22, &[]).await,
        Record::SocketEstablished(evidence) => {
            write_frame(output, 23, &encode_established(*evidence)?).await
        }
        Record::SocketRetired(evidence) => {
            write_frame(output, 24, &encode_retired(*evidence)?).await
        }
    }
}

async fn read_frame<R: AsyncRead + Unpin>(input: &mut R) -> io::Result<(u8, PacketBytes)> {
    let first = input.read_u8().await?;
    tokio::time::timeout(crate::IO_DEADLINE, read_frame_inner(input, first))
        .await
        .map_err(|_error| io::Error::new(io::ErrorKind::TimedOut, "capture IPC read deadline"))?
}

async fn read_frame_inner<R: AsyncRead + Unpin>(
    input: &mut R,
    first: u8,
) -> io::Result<(u8, PacketBytes)> {
    let mut header = [0; HEADER_BYTES];
    *header.first_mut().ok_or_else(invalid)? = first;
    input.read_exact(header.get_mut(1..).ok_or_else(invalid)?).await?;
    if header.get(..4) != Some(MAGIC.as_slice())
        || header.get(4) != Some(&VERSION)
        || header.get(6..8) != Some(&[0, 0])
    {
        return Err(invalid());
    }
    let kind = *header.get(5).ok_or_else(invalid)?;
    let length = u32::from_be_bytes(
        header.get(8..12).ok_or_else(invalid)?.try_into().map_err(|_error| invalid())?,
    ) as usize;
    // Check both global and per-kind limits before allocating or reading any body.
    let limit = match kind {
        1 | 2 | 19 | 21 | 22 => 0,
        17 => MAX_BODY_BYTES,
        18 => CONTROL_BYTES,
        16 | 20 => 1,
        23 => 53,
        24 => 49,
        _ => return Err(invalid()),
    };
    if length > limit {
        return Err(invalid());
    }
    let mut body = PacketBytes::new(vec![0; length]);
    input.read_exact(&mut body.0).await?;
    Ok((kind, body))
}

async fn write_frame<W: AsyncWrite + Unpin>(
    output: &mut W,
    kind: u8,
    body: &[u8],
) -> io::Result<()> {
    tokio::time::timeout(crate::IO_DEADLINE, write_frame_inner(output, kind, body))
        .await
        .map_err(|_error| io::Error::new(io::ErrorKind::TimedOut, "capture IPC write deadline"))?
}

async fn write_frame_inner<W: AsyncWrite + Unpin>(
    output: &mut W,
    kind: u8,
    body: &[u8],
) -> io::Result<()> {
    if body.len() > MAX_BODY_BYTES {
        return Err(invalid());
    }
    let mut header = Vec::with_capacity(HEADER_BYTES);
    header.extend(MAGIC);
    header.extend([VERSION, kind, 0, 0]);
    header.extend((body.len() as u32).to_be_bytes());
    output.write_all(&header).await?;
    output.write_all(body).await?;
    output.flush().await
}

fn exact_server_packet(bytes: &[u8]) -> bool {
    if bytes.len() > MAX_BODY_BYTES || packet::parse(bytes).is_none() {
        return false;
    }
    let Some(length_bytes) = (match bytes.first().map(|b| b >> 4) {
        Some(4) => bytes.get(2..4),
        Some(6) => bytes.get(4..6),
        _ => None,
    }) else {
        return false;
    };
    let Ok(length_bytes) = <[u8; 2]>::try_from(length_bytes) else {
        return false;
    };
    let declared = usize::from(u16::from_be_bytes(length_bytes))
        + if bytes.first().map(|b| b >> 4) == Some(6) { 40 } else { 0 };
    bytes.len() == declared
}

fn encode_control(control: ClientTcpControl) -> io::Result<Vec<u8>> {
    if !control.is_valid()
        || control.key.client.scope_id_nonzero()
        || control.key.server.scope_id_nonzero()
    {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(CONTROL_BYTES);
    bytes.push(if control.key.client.is_ipv4() { 4 } else { 6 });
    for address in [control.key.client, control.key.server] {
        match address.ip() {
            IpAddr::V4(ip) => {
                bytes.extend([0; 12]);
                bytes.extend(ip.octets());
            }
            IpAddr::V6(ip) => bytes.extend(ip.octets()),
        }
    }
    bytes.extend(control.key.client.port().to_be_bytes());
    bytes.extend(control.key.server.port().to_be_bytes());
    bytes.extend(control.sequence.to_be_bytes());
    bytes.extend(control.acknowledgement.to_be_bytes());
    bytes.push(control.flags);
    Ok(bytes)
}

// Scope/flowinfo cannot be reconstructed from packet bytes. Reject ambiguous input.
trait ScopeId {
    fn scope_id_nonzero(self) -> bool;
}
impl ScopeId for SocketAddr {
    fn scope_id_nonzero(self) -> bool {
        matches!(self, SocketAddr::V6(address) if address.scope_id() != 0 || address.flowinfo() != 0)
    }
}

fn decode_control(bytes: &[u8]) -> io::Result<ClientTcpControl> {
    if bytes.len() != CONTROL_BYTES {
        return Err(invalid());
    }
    let address = |offset: usize| -> io::Result<IpAddr> {
        let octets: [u8; 16] = bytes
            .get(offset..offset + 16)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_error| invalid())?;
        match bytes.first() {
            Some(4) if octets.get(..12) == Some(&[0; 12]) => Ok(IpAddr::V4(Ipv4Addr::from(
                <[u8; 4]>::try_from(octets.get(12..).ok_or_else(invalid)?)
                    .map_err(|_error| invalid())?,
            ))),
            Some(6) => Ok(IpAddr::V6(Ipv6Addr::from(octets))),
            _ => Err(invalid()),
        }
    };
    let port = |offset| -> io::Result<u16> {
        Ok(u16::from_be_bytes(
            bytes
                .get(offset..offset + 2)
                .ok_or_else(invalid)?
                .try_into()
                .map_err(|_error| invalid())?,
        ))
    };
    let number = |offset| -> io::Result<u32> {
        Ok(u32::from_be_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or_else(invalid)?
                .try_into()
                .map_err(|_error| invalid())?,
        ))
    };
    let control = ClientTcpControl {
        key: FlowKey {
            client: SocketAddr::new(address(1)?, port(33)?),
            server: SocketAddr::new(address(17)?, port(35)?),
        },
        sequence: number(37)?,
        acknowledgement: number(41)?,
        flags: *bytes.get(45).ok_or_else(invalid)?,
    };
    if control.is_valid() { Ok(control) } else { Err(invalid()) }
}

// Distinct schemas for OS socket attestations; these are never packet controls.
fn encode_evidence_key(key: FlowKey) -> io::Result<Vec<u8>> {
    if key.client.scope_id_nonzero() || key.server.scope_id_nonzero() {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(53);
    bytes.push(if key.client.is_ipv4() { 4 } else { 6 });
    for address in [key.client, key.server] {
        match address.ip() {
            IpAddr::V4(ip) => {
                bytes.extend([0; 12]);
                bytes.extend(ip.octets());
            }
            IpAddr::V6(ip) => bytes.extend(ip.octets()),
        }
    }
    bytes.extend(key.client.port().to_be_bytes());
    bytes.extend(key.server.port().to_be_bytes());
    Ok(bytes)
}
fn decode_evidence_key(bytes: &[u8]) -> io::Result<FlowKey> {
    let address = |offset| -> io::Result<IpAddr> {
        let octets: [u8; 16] = bytes
            .get(offset..offset + 16)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_error| invalid())?;
        match bytes.first() {
            Some(4) if octets.get(..12) == Some(&[0; 12]) => Ok(IpAddr::V4(Ipv4Addr::from(
                <[u8; 4]>::try_from(octets.get(12..).ok_or_else(invalid)?)
                    .map_err(|_error| invalid())?,
            ))),
            Some(6) => Ok(IpAddr::V6(Ipv6Addr::from(octets))),
            _ => Err(invalid()),
        }
    };
    let port = |offset| -> io::Result<u16> {
        Ok(u16::from_be_bytes(
            bytes
                .get(offset..offset + 2)
                .ok_or_else(invalid)?
                .try_into()
                .map_err(|_error| invalid())?,
        ))
    };
    Ok(FlowKey {
        client: SocketAddr::new(address(1)?, port(33)?),
        server: SocketAddr::new(address(17)?, port(35)?),
    })
}
fn number32(bytes: &[u8], offset: usize) -> io::Result<u32> {
    Ok(u32::from_be_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_error| invalid())?,
    ))
}
fn number64(bytes: &[u8], offset: usize) -> io::Result<u64> {
    Ok(u64::from_be_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or_else(invalid)?
            .try_into()
            .map_err(|_error| invalid())?,
    ))
}
fn encode_established(evidence: SocketEstablishedEvidence) -> io::Result<Vec<u8>> {
    if !evidence.is_valid() {
        return Err(invalid());
    }
    let mut bytes = encode_evidence_key(evidence.key)?;
    bytes.extend(evidence.client_initial_sequence.to_be_bytes());
    bytes.extend(evidence.server_initial_sequence.to_be_bytes());
    bytes.extend(evidence.generation.to_be_bytes());
    Ok(bytes)
}
fn decode_established(bytes: &[u8]) -> io::Result<SocketEstablishedEvidence> {
    if bytes.len() != 53 {
        return Err(invalid());
    }
    let evidence = SocketEstablishedEvidence {
        key: decode_evidence_key(bytes)?,
        client_initial_sequence: number32(bytes, 37)?,
        server_initial_sequence: number32(bytes, 41)?,
        generation: number64(bytes, 45)?,
    };
    if evidence.is_valid() { Ok(evidence) } else { Err(invalid()) }
}
fn encode_retired(evidence: SocketRetiredEvidence) -> io::Result<Vec<u8>> {
    if !evidence.is_valid() {
        return Err(invalid());
    }
    let mut bytes = encode_evidence_key(evidence.key)?;
    bytes.extend(evidence.client_initial_sequence.to_be_bytes());
    bytes.extend(evidence.generation.to_be_bytes());
    Ok(bytes)
}
fn decode_retired(bytes: &[u8]) -> io::Result<SocketRetiredEvidence> {
    if bytes.len() != 49 {
        return Err(invalid());
    }
    let evidence = SocketRetiredEvidence {
        key: decode_evidence_key(bytes)?,
        client_initial_sequence: number32(bytes, 37)?,
        generation: number64(bytes, 41)?,
    };
    if evidence.is_valid() { Ok(evidence) } else { Err(invalid()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn control() -> ClientTcpControl {
        ClientTcpControl {
            key: FlowKey {
                client: "192.0.2.1:45000".parse().expect("address"),
                server: "198.51.100.2:3101".parse().expect("address"),
            },
            sequence: 12,
            acknowledgement: 20,
            flags: 0x10,
        }
    }

    #[tokio::test]
    async fn records_round_trip_without_payload_in_debug() {
        let records = [
            Record::Started(Backend::WinDivert),
            Record::ClientControl(control()),
            Record::Gap,
            Record::Unavailable(UnavailableReason::OwnershipUnavailable),
            Record::Stopped,
        ];
        for record in records {
            let mut bytes = Vec::new();
            write_record(&mut bytes, &record).await.expect("write");
            assert_eq!(read_record(&mut bytes.as_slice()).await.expect("read"), record);
        }
        assert!(!format!("{:?}", Record::ClientControl(control())).contains("192.0.2"));
        assert!(
            !format!("{:?}", Record::ServerPacket(PacketBytes::new(b"secret".to_vec())))
                .contains("secret")
        );
    }

    #[tokio::test]
    async fn invalid_headers_are_rejected_without_reading_the_body() {
        for header in [
            *b"RKCI\x01\x11\0\0\0\x01\0\0",
            *b"RKCI\x02\x11\0\0\0\0\0\0",
            *b"RKCI\x01\x11\x01\0\0\0\0\0",
            *b"RKCI\x01\xff\0\0\0\0\0\0",
            *b"RKCI\x01\x10\0\0\0\0\0\x02",
        ] {
            assert_eq!(
                read_record(&mut header.as_slice()).await.expect_err("invalid header").kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[tokio::test]
    async fn truncated_frames_and_cross_direction_messages_fail() {
        let mut bytes = Vec::new();
        write_record(&mut bytes, &Record::ClientControl(control())).await.expect("write");
        for length in 0..bytes.len() {
            read_record(&mut bytes.get(..length).expect("prefix")).await.expect_err("truncated");
        }
        read_request(&mut bytes.as_slice()).await.expect_err("wrong direction");
        bytes.clear();
        write_request(&mut bytes, ClientRequest::Start).await.expect("start");
        read_record(&mut bytes.as_slice()).await.expect_err("wrong direction");
    }

    #[test]
    fn control_rejects_payload_tail_unknown_family_reserved_address_and_flags() {
        let bytes = encode_control(control()).expect("encode");
        let mut tail = bytes.clone();
        tail.push(1);
        decode_control(&tail).expect_err("extra data");
        for (offset, value) in [(0, 5), (1, 1), (45, 0x18)] {
            let mut bad = bytes.clone();
            *bad.get_mut(offset).expect("field") = value;
            decode_control(&bad).expect_err("invalid field");
        }
    }
    #[tokio::test(start_paused = true)]
    async fn partial_frame_and_stalled_write_have_finite_deadlines() {
        let (mut reader, mut writer) = tokio::io::duplex(1);
        writer.write_all(b"R").await.expect("first byte");
        let read = read_record(&mut reader);
        assert_eq!(read.await.expect_err("partial frame").kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            write_record(&mut writer, &Record::Started(Backend::WinDivert))
                .await
                .expect_err("stalled peer")
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[tokio::test]
    async fn server_packet_tail_client_data_and_oversize_are_rejected() {
        let mut bytes =
            vec![0x45, 0, 0, 41, 0, 0, 0, 0, 64, 6, 0, 0, 198, 51, 100, 1, 192, 0, 2, 2];
        bytes.extend(3101u16.to_be_bytes());
        bytes.extend(45000u16.to_be_bytes());
        bytes.extend([0; 8]);
        bytes.extend([0x50, 0x18, 0, 0, 0, 0, 0, 0, 1]);
        let mut encoded = Vec::new();
        write_record(&mut encoded, &Record::ServerPacket(PacketBytes::new(bytes.clone())))
            .await
            .expect("valid packet");
        assert_eq!(
            read_record(&mut encoded.as_slice()).await.expect("packet"),
            Record::ServerPacket(PacketBytes::new(bytes.clone()))
        );
        bytes.push(0);
        assert!(
            write_record(&mut Vec::new(), &Record::ServerPacket(PacketBytes::new(bytes)))
                .await
                .is_err()
        );
        assert!(
            write_record(
                &mut Vec::new(),
                &Record::ServerPacket(PacketBytes::new(vec![0; MAX_BODY_BYTES + 1]))
            )
            .await
            .is_err()
        );
    }
    #[tokio::test]
    async fn socket_evidence_round_trips_in_distinct_kinds_and_rejects_zero_generation() {
        let evidence = SocketEstablishedEvidence {
            key: control().key,
            client_initial_sequence: 100,
            server_initial_sequence: 200,
            generation: 1,
        };
        let retired = SocketRetiredEvidence {
            key: evidence.key,
            client_initial_sequence: 100,
            generation: 1,
        };
        for record in [Record::SocketEstablished(evidence), Record::SocketRetired(retired)] {
            let mut bytes = Vec::new();
            write_record(&mut bytes, &record).await.expect("write evidence");
            assert_eq!(read_record(&mut bytes.as_slice()).await.expect("evidence"), record);
            assert!(!format!("{record:?}").contains("192.0.2"));
            assert_ne!(bytes.get(5), Some(&18));
        }
        let invalid_evidence = SocketEstablishedEvidence { generation: 0, ..evidence };
        encode_established(invalid_evidence).expect_err("zero generation");
        let mut extra = encode_retired(retired).expect("retire");
        extra.push(0);
        decode_retired(&extra).expect_err("extra bytes");
    }
    #[tokio::test]
    async fn authenticated_session_rejects_heartbeat_evidence_and_data_before_started() {
        for first in [Record::Keepalive, Record::Gap, Record::ClientControl(control())] {
            let mut bytes = Vec::new();
            write_record(&mut bytes, &first).await.expect("synthetic source");
            write_record(&mut bytes, &Record::Started(Backend::Pcap)).await.expect("late start");
            let mut reader = SessionReader::new(bytes.as_slice());
            assert_eq!(
                reader.read().await.expect_err("pre-start record").kind(),
                io::ErrorKind::InvalidData
            );
            reader.read().await.expect_err("session remains closed");
        }
        let mut bytes = Vec::new();
        write_record(&mut bytes, &Record::Started(Backend::Pcap)).await.expect("start");
        write_record(&mut bytes, &Record::Keepalive).await.expect("heartbeat");
        write_record(&mut bytes, &Record::Stopped).await.expect("stop");
        let mut reader = SessionReader::new(bytes.as_slice());
        assert_eq!(reader.read().await.expect("first"), Record::Started(Backend::Pcap));
        assert_eq!(reader.read().await.expect("next"), Record::Keepalive);
        assert_eq!(reader.read().await.expect("last"), Record::Stopped);
        reader.read().await.expect_err("terminated");
        let mut unavailable = Vec::new();
        write_record(&mut unavailable, &Record::Unavailable(UnavailableReason::NativeBackend))
            .await
            .expect("unavailable");
        assert!(matches!(
            SessionReader::new(unavailable.as_slice()).read().await.expect("startup failure"),
            Record::Unavailable(_)
        ));
    }
}
