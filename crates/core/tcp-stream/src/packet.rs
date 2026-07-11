//! TCP payload extraction for common libpcap datalink formats.

use std::net::IpAddr;

/// TCP payload pulled from a captured packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpPayload {
    /// Source IP address.
    pub source_addr: IpAddr,
    /// Source TCP port.
    pub source_port: u16,
    /// Destination IP address.
    pub destination_addr: IpAddr,
    /// Destination TCP port.
    pub destination_port: u16,
    /// True when the segment has FIN set.
    pub fin: bool,
    /// True when the segment has RST set.
    pub rst: bool,
    /// Application bytes after the TCP/IP headers.
    pub payload: Vec<u8>,
}

/// Parse link-layer packet data and return an IPv4/IPv6 TCP payload.
///
/// libpcap reports different datalink ids depending on OS and adapter. This
/// covers the formats we can decode without pulling pcap details into callers.
pub fn parse_tcp_packet(link_type: i32, packet: &[u8]) -> Option<TcpPayload> {
    let ip_packet = match link_type {
        1 => packet.get(14..)?,
        7 | 12 | 101 | 228 => packet,
        113 => packet.get(16..)?,
        0 => {
            let family = u32::from_ne_bytes(packet.get(0..4)?.try_into().ok()?);
            let payload = packet.get(4..)?;
            match family {
                2 | 24 | 28 | 30 => payload,
                _ => return None,
            }
        }
        _ => return None,
    };

    match ip_packet.first()? >> 4 {
        4 => parse_ipv4_tcp(ip_packet),
        6 => parse_ipv6_tcp(ip_packet),
        _ => None,
    }
}

fn parse_ipv4_tcp(packet: &[u8]) -> Option<TcpPayload> {
    let header_len = usize::from(packet.first()? & 0x0f) * 4;
    if header_len < 20 || packet.len() < header_len {
        return None;
    }
    if *packet.get(9)? != 6 {
        return None;
    }

    let total_len = usize::from(u16::from_be_bytes(packet.get(2..4)?.try_into().ok()?));
    let packet = packet.get(..total_len.min(packet.len()))?;
    let source_addr = IpAddr::from(<[u8; 4]>::try_from(packet.get(12..16)?).ok()?);
    let destination_addr = IpAddr::from(<[u8; 4]>::try_from(packet.get(16..20)?).ok()?);
    parse_tcp_segment(source_addr, destination_addr, packet.get(header_len..)?)
}

fn parse_ipv6_tcp(packet: &[u8]) -> Option<TcpPayload> {
    if packet.len() < 40 || *packet.get(6)? != 6 {
        return None;
    }

    let payload_len = usize::from(u16::from_be_bytes(packet.get(4..6)?.try_into().ok()?));
    let total_len = 40usize.checked_add(payload_len)?;
    let packet = packet.get(..total_len.min(packet.len()))?;
    let source_addr = IpAddr::from(<[u8; 16]>::try_from(packet.get(8..24)?).ok()?);
    let destination_addr = IpAddr::from(<[u8; 16]>::try_from(packet.get(24..40)?).ok()?);
    parse_tcp_segment(source_addr, destination_addr, packet.get(40..)?)
}

fn parse_tcp_segment(
    source_addr: IpAddr,
    destination_addr: IpAddr,
    segment: &[u8],
) -> Option<TcpPayload> {
    if segment.len() < 20 {
        return None;
    }

    let source_port = u16::from_be_bytes(segment.get(0..2)?.try_into().ok()?);
    let destination_port = u16::from_be_bytes(segment.get(2..4)?.try_into().ok()?);
    let data_offset = usize::from(segment.get(12)? >> 4) * 4;
    if data_offset < 20 || segment.len() < data_offset {
        return None;
    }
    let flags = *segment.get(13)?;

    Some(TcpPayload {
        source_addr,
        source_port,
        destination_addr,
        destination_port,
        fin: flags & 0x01 != 0,
        rst: flags & 0x04 != 0,
        payload: segment[data_offset..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_packet_should_extract_raw_ipv4_tcp_payload() {
        let packet = raw_ipv4_tcp_packet(56_380, 3101, &[0, 2, 0xab, 0xcd]);

        let parsed = parse_tcp_packet(101, &packet);

        assert_eq!(parsed.map(|payload| payload.payload), Some(vec![0, 2, 0xab, 0xcd]));
    }

    #[test]
    fn parse_tcp_packet_should_reject_unsupported_link_type() {
        let parsed = parse_tcp_packet(999, &[0, 1, 2, 3]);

        assert_eq!(parsed, None);
    }

    fn raw_ipv4_tcp_packet(source_port: u16, destination_port: u16, payload: &[u8]) -> Vec<u8> {
        let total_len = 20 + 20 + payload.len();
        let mut packet = vec![0u8; total_len];
        packet[0] = 0x45;
        packet[2..4].copy_from_slice(&u16::try_from(total_len).unwrap().to_be_bytes());
        packet[9] = 6;
        packet[12..16].copy_from_slice(&[10, 0, 0, 1]);
        packet[16..20].copy_from_slice(&[10, 0, 0, 2]);
        packet[20..22].copy_from_slice(&source_port.to_be_bytes());
        packet[22..24].copy_from_slice(&destination_port.to_be_bytes());
        packet[32] = 5 << 4;
        packet[40..].copy_from_slice(payload);
        packet
    }
}
