//! A small admission check, not a protocol decoder or stream reassembler.
//! Reject ambiguous/truncated/fragmented data rather than exposing client bytes.

pub(crate) fn server_packet(ip: &[u8]) -> Option<&[u8]> {
    let (length, tcp_offset) = match ip.first()? >> 4 {
        4 => {
            let header = usize::from(ip.first()? & 0x0f) * 4;
            let length = usize::from(be16(ip, 2)?);

            // No IPv4 fragments (including a first fragment with more to follow).
            if header < 20 || ip.get(9) != Some(&6) || be16(ip, 6)? & 0x3fff != 0 {
                return None;
            }

            (length, header)
        }
        6 => {
            // Extension headers, fragments and jumbograms are deliberately not
            // admitted in this adapter stage. Never guess a TCP header offset.
            if ip.get(6) != Some(&6) || be16(ip, 4)? == 0 {
                return None;
            }

            (40 + usize::from(be16(ip, 4)?), 40)
        }
        _ => return None,
    };

    let ip = ip.get(..length)?;
    let tcp = ip.get(tcp_offset..)?;
    let tcp_header = usize::from(tcp.get(12)? >> 4) * 4;
    if tcp_header < 20 || tcp.len() < tcp_header || !server_port(be16(tcp, 0)?) {
        return None;
    }

    // Exclude ambiguous server-port-to-server-port traffic as well.
    if server_port(be16(tcp, 2)?) {
        return None;
    }

    Some(ip)
}

pub(crate) fn destination(ip: &[u8]) -> Option<std::net::IpAddr> {
    match ip.first()? >> 4 {
        4 => Some(std::net::IpAddr::from(<[u8; 4]>::try_from(ip.get(16..20)?).ok()?)),
        6 => Some(std::net::IpAddr::from(<[u8; 16]>::try_from(ip.get(24..40)?).ok()?)),
        _ => None,
    }
}

fn server_port(port: u16) -> bool {
    matches!(port, 3101 | 5222)
}

fn be16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn ipv4() -> Vec<u8> {
        let mut bytes = vec![0; 41];
        bytes[0] = 0x45;
        bytes[2..4].copy_from_slice(&41_u16.to_be_bytes());
        bytes[9] = 6;
        bytes[12..16].copy_from_slice(&[198, 51, 100, 1]);
        bytes[16..20].copy_from_slice(&[192, 0, 2, 2]);
        bytes[20..22].copy_from_slice(&3101_u16.to_be_bytes());
        bytes[22..24].copy_from_slice(&45000_u16.to_be_bytes());
        bytes[32] = 0x50;
        bytes[40] = 42;

        bytes
    }

    #[test]
    fn admits_server_payload_and_strips_padding() {
        let mut bytes = ipv4();
        bytes.extend([0; 12]);

        assert_eq!(server_packet(&bytes).map(<[u8]>::len), Some(41));
    }

    #[test]
    fn both_ip_versions_admit_exactly_the_two_server_source_ports() {
        let mut v4 = ipv4();
        let mut v6 = vec![0; 40];
        v6[0] = 0x60;
        v6[4..6].copy_from_slice(&21_u16.to_be_bytes());
        v6[6] = 6;
        v6.extend_from_slice(&v4[20..]);

        for (bytes, offset) in [(&mut v4, 20), (&mut v6, 40)] {
            for port in 0..=u16::MAX {
                bytes[offset..offset + 2].copy_from_slice(&port.to_be_bytes());
                assert_eq!(
                    server_packet(bytes).is_some(),
                    [3101, 5222].contains(&port),
                    "source port {port} at TCP offset {offset}"
                );
            }
        }
    }

    #[test]
    fn both_server_ports_reject_client_payloads_and_ambiguous_cross_pairs() {
        for server in [3101_u16, 5222] {
            let mut bytes = ipv4();
            bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
            bytes[22..24].copy_from_slice(&server.to_be_bytes());
            assert_eq!(server_packet(&bytes), None, "client to {server}");

            bytes[20..22].copy_from_slice(&server.to_be_bytes());
            for destination in [3101_u16, 5222] {
                bytes[22..24].copy_from_slice(&destination.to_be_bytes());
                assert_eq!(server_packet(&bytes), None, "{server} to {destination}");
            }
        }
    }

    #[test]
    fn both_ports_retain_empty_server_syn_fin_and_reset_segments() {
        let mut v4 = ipv4();
        v4.truncate(40);
        v4[2..4].copy_from_slice(&40_u16.to_be_bytes());
        let mut v6 = vec![0; 40];
        v6[0] = 0x60;
        v6[4..6].copy_from_slice(&20_u16.to_be_bytes());
        v6[6] = 6;
        v6.extend_from_slice(&v4[20..]);

        for (bytes, offset) in [(&mut v4, 20), (&mut v6, 40)] {
            for port in [3101_u16, 5222] {
                bytes[offset..offset + 2].copy_from_slice(&port.to_be_bytes());
                for flags in [0x02, 0x12, 0x11, 0x14] {
                    bytes[offset + 13] = flags;
                    assert_eq!(server_packet(bytes), Some(bytes.as_slice()));
                }
            }
        }
    }

    #[test]
    fn rejects_every_truncated_prefix() {
        let bytes = ipv4();

        for end in 0..bytes.len() {
            assert_eq!(server_packet(&bytes[..end]), None);
        }
    }

    #[test]
    fn rejects_clients_fragments_other_protocols_and_invalid_headers() {
        for (offset, replacement) in [(20, 0), (6, 0x20), (7, 1), (9, 17), (32, 0xf0), (0, 0x44)] {
            let mut bytes = ipv4();
            bytes[offset] = replacement;
            assert_eq!(server_packet(&bytes), None, "offset {offset}");
        }

        let mut bytes = ipv4();
        bytes[2..4].copy_from_slice(&40_u16.to_be_bytes());

        assert_eq!(server_packet(&bytes).map(<[u8]>::len), Some(40));

        bytes = ipv4();
        bytes[22..24].copy_from_slice(&3101_u16.to_be_bytes());
        assert_eq!(server_packet(&bytes), None);
    }

    #[test]
    fn accepts_base_ipv6_tcp_and_rejects_extensions_or_jumbograms() {
        let v4 = ipv4();
        let mut v6 = vec![0; 40];
        v6[0] = 0x60;
        v6[4..6].copy_from_slice(&21_u16.to_be_bytes());
        v6[6] = 6;
        v6.extend_from_slice(&v4[20..]);

        assert_eq!(server_packet(&v6), Some(v6.as_slice()));

        for next_header in [0, 43, 44, 60, 17] {
            v6[6] = next_header;
            assert_eq!(server_packet(&v6), None);
        }

        v6[6] = 6;
        v6[4..6].fill(0);
        assert_eq!(server_packet(&v6), None);
    }
}
