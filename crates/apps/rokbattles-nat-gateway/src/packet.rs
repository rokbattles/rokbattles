//! Parses complete IPv4 TCP packets copied after conntrack defragmentation.

use std::net::{Ipv4Addr, SocketAddrV4};

/// Borrowed TCP fields needed for passive stream reconstruction.
#[derive(Debug)]
pub struct Packet<'a> {
    pub source: SocketAddrV4,
    pub destination: SocketAddrV4,
    pub sequence: u32,
    pub acknowledgement: u32,
    pub flags: u8,
    pub payload: &'a [u8],
}

/// Read an IPv4 TCP packet, rejecting truncation, fragments and bad lengths.
/// Checksums are validated by the forwarding stack; offload can leave the
/// copied checksum incomplete, so it is not recalculated here.
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
}
