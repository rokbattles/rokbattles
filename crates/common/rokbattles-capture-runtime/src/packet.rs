//! Validate raw IP packets again at the unprivileged worker boundary.

use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use crate::SERVER_PORTS;

/// Exact local-client/remote-server tuple. Different client ports never share state.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlowKey {
    pub client: SocketAddr,
    pub server: SocketAddr,
}

impl fmt::Debug for FlowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FlowKey").finish_non_exhaustive()
    }
}

/// Header-only client observation. Addresses, sequence numbers and flags stay local.
/// There is no payload field or ingress serialization implementation.
///
/// ```compile_fail
/// use rokbattles_capture_runtime::{packet::ClientTcpControl, wire};
/// fn cannot_upload(control: &ClientTcpControl) {
///     let _ = wire::encode(control);
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClientTcpControl {
    pub key: FlowKey,
    pub sequence: u32,
    pub acknowledgement: u32,
    pub flags: u8,
}

impl ClientTcpControl {
    /// Revalidate metadata at each local trust boundary. Native adapters separately
    /// prove the complete TCP segment contains zero payload before constructing it.
    pub fn is_valid(self) -> bool {
        valid_flow_key(self.key)
            && matches!(self.flags & 0x3f, 0x01 | 0x02 | 0x04 | 0x10 | 0x11 | 0x14)
    }
}

impl fmt::Debug for ClientTcpControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientTcpControl").finish_non_exhaustive()
    }
}

/// A validated, borrowed server TCP packet. Debug deliberately omits addresses/payloads.
pub struct ServerPacket<'a> {
    pub(crate) key: FlowKey,
    pub(crate) sequence: u32,
    pub(crate) acknowledgement: u32,
    pub(crate) flags: u8,
    pub(crate) payload: &'a [u8],
}

impl ServerPacket<'_> {
    /// Local routing metadata; never include this tuple in ingress messages.
    pub fn flow_key(&self) -> FlowKey {
        self.key
    }

    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    pub fn acknowledgement(&self) -> u32 {
        self.acknowledgement
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }
}

impl fmt::Debug for ServerPacket<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerPacket")
            .field("flags", &self.flags)
            .field("payload_bytes", &self.payload.len())
            .finish_non_exhaustive()
    }
}

/// Match the current adapters' admission policy: complete IPv4 or plain IPv6 TCP,
/// no fragments, no IPv6 extensions, no client payload or server-to-server traffic.
/// Offloaded checksums are not validated here.
pub fn parse(bytes: &[u8]) -> Option<ServerPacket<'_>> {
    let (source, destination, offset, length) = match bytes.first()? >> 4 {
        4 => {
            let header = usize::from(bytes.first()? & 0x0f) * 4;
            let length = usize::from(be16(bytes, 2)?);

            if header < 20 || bytes.get(9) != Some(&6) || be16(bytes, 6)? & 0x3fff != 0 {
                return None;
            }

            (
                IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(bytes.get(12..16)?).ok()?)),
                IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(bytes.get(16..20)?).ok()?)),
                header,
                length,
            )
        }
        6 => {
            if bytes.get(6) != Some(&6) || be16(bytes, 4)? == 0 {
                return None;
            }

            (
                IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(bytes.get(8..24)?).ok()?)),
                IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(bytes.get(24..40)?).ok()?)),
                40,
                40 + usize::from(be16(bytes, 4)?),
            )
        }
        _ => return None,
    };

    let tcp = bytes.get(..length)?.get(offset..)?;
    let server_port = be16(tcp, 0)?;
    let client_port = be16(tcp, 2)?;
    let header = usize::from(tcp.get(12)? >> 4) * 4;

    if !SERVER_PORTS.contains(&server_port)
        || SERVER_PORTS.contains(&client_port)
        || client_port == 0
        || header < 20
    {
        return None;
    }

    let key = FlowKey {
        client: SocketAddr::new(destination, client_port),
        server: SocketAddr::new(source, server_port),
    };
    if !valid_flow_key(key) {
        return None;
    }

    Some(ServerPacket {
        key,
        sequence: u32::from_be_bytes(tcp.get(4..8)?.try_into().ok()?),
        acknowledgement: u32::from_be_bytes(tcp.get(8..12)?.try_into().ok()?),
        flags: *tcp.get(13)?,
        payload: tcp.get(header..)?,
    })
}

pub(crate) fn valid_flow_key(key: FlowKey) -> bool {
    fn unicast(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ip) => !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast(),
            IpAddr::V6(ip) => !ip.is_unspecified() && !ip.is_multicast(),
        }
    }

    key.client.is_ipv4() == key.server.is_ipv4()
        && SERVER_PORTS.contains(&key.server.port())
        && key.client.port() != 0
        && !SERVER_PORTS.contains(&key.client.port())
        && unicast(key.client.ip())
        && unicast(key.server.ip())
}

fn be16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Vec<u8> {
        let mut bytes = vec![0x45, 0];
        bytes.extend(46_u16.to_be_bytes());
        bytes.extend([0, 0, 0, 0, 64, 6, 0, 0, 198, 51, 100, 1, 192, 0, 2, 2]);
        bytes.extend(3101_u16.to_be_bytes());
        bytes.extend(45000_u16.to_be_bytes());
        bytes.extend(100_u32.to_be_bytes());
        bytes.extend(200_u32.to_be_bytes());
        bytes.extend([0x50, 0x18, 0, 0, 0, 0, 0, 0]);
        bytes.extend(b"secret");
        bytes
    }

    #[test]
    fn every_truncated_prefix_is_rejected_and_padding_excluded() {
        let mut bytes = packet();

        for length in 0..bytes.len() {
            assert!(parse(bytes.get(..length).expect("test prefix")).is_none());
        }

        bytes.extend(b"padding");
        assert_eq!(parse(&bytes).expect("packet").payload, b"secret");
    }

    #[test]
    fn both_ports_admit_only_server_payload_and_debug_is_redacted() {
        for server_port in SERVER_PORTS {
            let mut bytes = packet();
            bytes.get_mut(20..22).expect("port").copy_from_slice(&server_port.to_be_bytes());
            let parsed = parse(&bytes).expect("server packet");
            assert_eq!(parsed.key.server.port(), server_port);
            assert!(!format!("{parsed:?}").contains("secret"));

            bytes.get_mut(20..22).expect("port").copy_from_slice(&45000_u16.to_be_bytes());
            bytes.get_mut(22..24).expect("port").copy_from_slice(&server_port.to_be_bytes());
            assert!(parse(&bytes).is_none());
        }
    }

    #[test]
    fn rejects_fragments_ipv6_extensions_and_ambiguous_ports() {
        let mut bytes = packet();
        bytes.get_mut(6..8).expect("fragment").copy_from_slice(&0x2000_u16.to_be_bytes());
        assert!(parse(&bytes).is_none());

        bytes.get_mut(6..8).expect("fragment").fill(0);
        bytes.get_mut(22..24).expect("port").copy_from_slice(&5222_u16.to_be_bytes());
        assert!(parse(&bytes).is_none());

        let tcp = packet();
        let mut ipv6 = vec![0u8; 40];
        *ipv6.first_mut().expect("version") = 0x60;
        ipv6.get_mut(4..6).expect("length").copy_from_slice(&26_u16.to_be_bytes());
        *ipv6.get_mut(6).expect("next header") = 6;
        *ipv6.get_mut(23).expect("source") = 1;
        *ipv6.get_mut(39).expect("destination") = 2;
        ipv6.extend(tcp.get(20..).expect("TCP"));
        assert!(parse(&ipv6).is_some());
        *ipv6.get_mut(6).expect("next header") = 44;
        assert!(parse(&ipv6).is_none());
    }
}
