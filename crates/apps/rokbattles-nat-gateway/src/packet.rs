//! Borrow the TCP fields needed to observe an IPv4 connection.
//!
//! Input is an entire layer-3 packet copied by NFLOG after conntrack's fragment
//! handling. Ethernet headers are not expected. Parsing bounds every slice by
//! the IPv4 total length and TCP data offset; trailing capture padding is ignored.
//! Checksums are not verified because packet copies may contain offload state.

use std::net::{Ipv4Addr, SocketAddrV4};

/// A parsed TCP segment whose payload borrows from the captured packet.
///
/// Sequence numbers remain in TCP's wrapping 32-bit number space. The observer
/// interprets SYN and FIN separately from the payload bytes.
#[derive(Debug)]
pub struct Packet<'a> {
    /// Source IPv4 address and TCP port as seen at the capture hook.
    pub source: SocketAddrV4,
    /// Destination IPv4 address and TCP port after applicable NAT translation.
    pub destination: SocketAddrV4,
    /// TCP sequence number, including any sequence space occupied by SYN.
    pub sequence: u32,
    /// Raw acknowledgement number; meaningful only when ACK is set.
    pub acknowledgement: u32,
    /// TCP flag octet containing FIN, SYN, RST, PSH, ACK, URG, ECE, and CWR.
    pub flags: u8,
    /// Bytes after TCP options and before the IPv4 total-length boundary.
    pub payload: &'a [u8],
}

/// Parse a complete IPv4/TCP packet without copying its payload.
///
/// Returns `None` for other IP versions or protocols, truncated headers, invalid
/// header lengths, or packets with a fragment offset or the more-fragments bit.
/// IPv4 and TCP options are skipped using their declared header lengths. The
/// don't-fragment bit is accepted; checksums and option contents are not checked.
pub fn parse(bytes: &[u8]) -> Option<Packet<'_>> {
    let first = *bytes.first()?;
    if first >> 4 != 4 || *bytes.get(9)? != 6 {
        return None;
    }
    let header = usize::from(first & 15) * 4;
    let length = usize::from(u16::from_be_bytes(bytes.get(2..4)?.try_into().ok()?));
    if header < 20 || length < header + 20 || length > bytes.len() {
        return None;
    }
    let fragment = u16::from_be_bytes(bytes.get(6..8)?.try_into().ok()?);
    if fragment & 0x3fff != 0 {
        return None;
    }
    let source = Ipv4Addr::from(<[u8; 4]>::try_from(bytes.get(12..16)?).ok()?);
    let destination = Ipv4Addr::from(<[u8; 4]>::try_from(bytes.get(16..20)?).ok()?);
    let tcp = bytes.get(header..length)?;
    let tcp_header = usize::from(*tcp.get(12)? >> 4) * 4;
    if tcp_header < 20 {
        return None;
    }
    Some(Packet {
        source: SocketAddrV4::new(source, u16::from_be_bytes(tcp.get(..2)?.try_into().ok()?)),
        destination: SocketAddrV4::new(
            destination,
            u16::from_be_bytes(tcp.get(2..4)?.try_into().ok()?),
        ),
        sequence: u32::from_be_bytes(tcp.get(4..8)?.try_into().ok()?),
        acknowledgement: u32::from_be_bytes(tcp.get(8..12)?.try_into().ok()?),
        flags: *tcp.get(13)?,
        payload: tcp.get(tcp_header..)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0x45, 0];
        bytes.extend(u16::try_from(40 + payload.len()).expect("test packet length").to_be_bytes());
        bytes.extend([0, 1, 0, 0, 64, 6, 0, 0]);
        bytes.extend([192, 0, 2, 1]);
        bytes.extend([198, 51, 100, 2]);
        bytes.extend(45_000_u16.to_be_bytes());
        bytes.extend(3_101_u16.to_be_bytes());
        bytes.extend(123_u32.to_be_bytes());
        bytes.extend(456_u32.to_be_bytes());
        bytes.extend([0x50, 0x18, 0xff, 0xff, 0, 0, 0, 0]);
        bytes.extend(payload);
        bytes
    }

    #[test]
    fn valid_packet_exposes_only_declared_payload() {
        let mut bytes = packet(b"mail");
        bytes.extend(b"netlink padding");

        let parsed = parse(&bytes).expect("packet should parse");

        assert_eq!(parsed.source, "192.0.2.1:45000".parse().expect("source"));
        assert_eq!(parsed.destination, "198.51.100.2:3101".parse().expect("destination"));
        assert_eq!((parsed.sequence, parsed.acknowledgement, parsed.flags), (123, 456, 0x18));
        assert_eq!(parsed.payload, b"mail");
    }

    #[test]
    fn malformed_headers_and_fragments_are_rejected() {
        let base = packet(b"mail");
        let mutations: [fn(&mut Vec<u8>); 9] = [
            |bytes| *bytes.get_mut(0).expect("version byte") = 0x65,
            |bytes| *bytes.get_mut(0).expect("IHL byte") = 0x44,
            |bytes| *bytes.get_mut(9).expect("protocol byte") = 17,
            |bytes| bytes.get_mut(2..4).expect("length").copy_from_slice(&39_u16.to_be_bytes()),
            |bytes| {
                bytes.get_mut(2..4).expect("length").copy_from_slice(&65_535_u16.to_be_bytes());
            },
            |bytes| {
                bytes.get_mut(6..8).expect("fragment").copy_from_slice(&0x2000_u16.to_be_bytes());
            },
            |bytes| {
                bytes.get_mut(6..8).expect("fragment").copy_from_slice(&1_u16.to_be_bytes());
            },
            |bytes| *bytes.get_mut(32).expect("TCP offset byte") = 0x40,
            |bytes: &mut Vec<u8>| bytes.truncate(19),
        ];
        for mutation in mutations {
            let mut bytes = base.clone();
            mutation(&mut bytes);
            assert!(parse(&bytes).is_none());
        }
    }

    #[test]
    fn every_truncated_prefix_is_rejected_without_panicking() {
        let bytes = packet(b"mail");
        for length in 0..bytes.len() {
            assert!(parse(bytes.get(..length).expect("test prefix")).is_none(), "length {length}");
        }
    }

    #[test]
    fn ipv4_and_tcp_options_do_not_leak_into_payload() {
        let mut bytes = packet(b"mail");
        bytes.splice(20..20, [1, 1, 1, 1]);
        bytes[0] = 0x46;
        bytes[36] = 0x60;
        bytes.splice(44..44, [1, 1, 1, 1]);
        let total = u16::try_from(bytes.len()).expect("packet length");
        bytes[2..4].copy_from_slice(&total.to_be_bytes());
        bytes.extend(b"capture padding");

        let parsed = parse(&bytes).expect("options fit declared lengths");
        assert_eq!(parsed.sequence, 123);
        assert_eq!(parsed.payload, b"mail");
    }

    #[test]
    fn dont_fragment_is_accepted_but_tcp_offsets_must_fit() {
        let mut bytes = packet(b"mail");
        bytes[6..8].copy_from_slice(&0x4000_u16.to_be_bytes());
        assert_eq!(parse(&bytes).expect("unfragmented DF packet").payload, b"mail");

        bytes[32] = 0xf0;
        assert!(parse(&bytes).is_none(), "TCP options exceed IP packet length");
    }
}
