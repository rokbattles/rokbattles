//! Receives passive NFLOG copies. The socket never supplies packet verdicts.

use std::{
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use crate::rules::NFLOG_GROUP;

/// A bound NFLOG group. Closing it leaves forwarding rules and conntrack intact.
///
/// Keeping this descriptor after privilege removal does not retain netfilter
/// write authority. Linux routes every `NETLINK_NETFILTER` command through
/// [`nfnetlink_rcv()`], which requires `CAP_NET_ADMIN` through
/// [`netlink_net_capable()`]. That check requires both the opener's file
/// credential and the current sender's capability, so the service's verified
/// all-zero capability sets make this descriptor receive-only in practice.
///
/// [`nfnetlink_rcv()`]: https://github.com/torvalds/linux/blob/master/net/netfilter/nfnetlink.c
/// [`netlink_net_capable()`]: https://github.com/torvalds/linux/blob/master/net/netlink/af_netlink.c
pub struct Capture {
    socket: OwnedFd,
}

fn os_result(result: i32) -> io::Result<()> {
    if result < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

impl Capture {
    /// Bind the exclusive group before installing rules or dropping privileges.
    pub fn open() -> io::Result<Self> {
        // SAFETY: socket takes integer constants and returns a new owned fd.
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                libc::NETLINK_NETFILTER,
            )
        };
        os_result(fd)?;
        // SAFETY: fd is a newly created socket and has no other owner.
        let socket = unsafe { OwnedFd::from_raw_fd(fd) };
        // SAFETY: all-zero sockaddr_nl is valid before setting its family.
        let mut address: libc::sockaddr_nl = unsafe { mem::zeroed() };
        address.nl_family = libc::AF_NETLINK as u16;
        // SAFETY: address is a valid initialized sockaddr_nl of the supplied size.
        os_result(unsafe {
            libc::bind(fd, (&raw const address).cast(), mem::size_of_val(&address) as u32)
        })?;
        let size: i32 = 4 * 1024 * 1024;
        // A normal SO_RCVBUF request is silently capped by the host's rmem_max.
        // Startup already needs NET_ADMIN, so request this socket's bounded
        // allocation directly without changing a global sysctl.
        // SAFETY: size is a valid integer socket option value.
        let forced = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUFFORCE,
                (&raw const size).cast(),
                mem::size_of_val(&size) as u32,
            )
        };
        if forced < 0 {
            // Some user-namespace kernels reserve FORCE for initial-namespace
            // root. Capture remains useful with the ordinary socket bound.
            // SAFETY: size remains a valid integer socket option value.
            os_result(unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_RCVBUF,
                    (&raw const size).cast(),
                    mem::size_of_val(&size) as u32,
                )
            })?;
        }
        let mut actual: i32 = 0;
        let mut option_len = mem::size_of_val(&actual) as u32;
        // SAFETY: actual and option_len are initialized writable outputs.
        os_result(unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                (&raw mut actual).cast(),
                &raw mut option_len,
            )
        })?;
        tracing::info!(receive_buffer_bytes = actual, "NFLOG receive buffer configured");
        let capture = Self { socket };
        capture.configure(1, &[1], 1)?; // NFULA_CFG_CMD / BIND
        let mut mode = 65535_u32.to_be_bytes().to_vec();
        mode.extend([2, 0]); // NFULNL_COPY_PACKET
        capture.configure(2, &mode, 2)?;
        capture.configure(6, &5_u16.to_be_bytes(), 3)?; // local sequence + conntrack info
        Ok(capture)
    }

    fn configure(&self, kind: u16, value: &[u8], sequence: u32) -> io::Result<()> {
        let attr_len = 4 + value.len();
        let length = 20 + attr_len.next_multiple_of(4);
        let mut message = Vec::with_capacity(length);
        message.extend((length as u32).to_ne_bytes());
        message.extend(0x401_u16.to_ne_bytes());
        message.extend(5_u16.to_ne_bytes()); // REQUEST | ACK
        message.extend(sequence.to_ne_bytes());
        message.extend(0_u32.to_ne_bytes());
        message.extend([libc::AF_INET as u8, 0]);
        message.extend(NFLOG_GROUP.to_be_bytes());
        message.extend((attr_len as u16).to_ne_bytes());
        message.extend(kind.to_ne_bytes());
        message.extend(value);
        message.resize(length, 0);
        // SAFETY: zero initializes all fields, including padding.
        let mut kernel: libc::sockaddr_nl = unsafe { mem::zeroed() };
        kernel.nl_family = libc::AF_NETLINK as u16;
        // SAFETY: message and destination remain valid for the duration of sendto.
        let sent = unsafe {
            libc::sendto(
                self.socket.as_raw_fd(),
                message.as_ptr().cast(),
                message.len(),
                0,
                (&raw const kernel).cast(),
                mem::size_of_val(&kernel) as u32,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut buffer = vec![0; 131_072];
        for _ in 0..32 {
            let count = self.receive(&mut buffer)?;
            let datagram = buffer
                .get(..count)
                .ok_or_else(|| io::Error::other("invalid NFLOG datagram length"))?;
            if let Some(code) = acknowledgement(datagram, sequence)? {
                return if code == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(-code)) };
            }
        }
        Err(io::Error::other("NFLOG acknowledgement was not received"))
    }

    /// Read a kernel datagram. ENOBUFS or truncation means observation lost data.
    pub fn receive(&self, buffer: &mut [u8]) -> io::Result<usize> {
        // SAFETY: zero initializes all sockaddr fields.
        let mut source: libc::sockaddr_nl = unsafe { mem::zeroed() };
        let mut length = mem::size_of_val(&source) as libc::socklen_t;
        // SAFETY: buffer is writable for its length; source and length are valid outputs.
        let result = unsafe {
            libc::recvfrom(
                self.socket.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                libc::MSG_TRUNC,
                (&raw mut source).cast(),
                &raw mut length,
            )
        };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        if source.nl_pid != 0 || result.cast_unsigned() > buffer.len() {
            return Err(io::Error::other("untrusted or truncated NFLOG datagram"));
        }
        Ok(result.cast_unsigned())
    }
}

fn acknowledgement(bytes: &[u8], expected_sequence: u32) -> io::Result<Option<i32>> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let length = remaining
            .get(..4)
            .and_then(|value| <[u8; 4]>::try_from(value).ok())
            .map(u32::from_ne_bytes)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| io::Error::other("short netlink acknowledgement header"))?;
        if length < 16 {
            return Err(io::Error::other("invalid netlink acknowledgement length"));
        }
        let message = remaining
            .get(..length)
            .ok_or_else(|| io::Error::other("truncated netlink acknowledgement"))?;
        let kind = u16::from_ne_bytes(
            message
                .get(4..6)
                .and_then(|value| <[u8; 2]>::try_from(value).ok())
                .ok_or_else(|| io::Error::other("short netlink acknowledgement type"))?,
        );
        let sequence = u32::from_ne_bytes(
            message
                .get(8..12)
                .and_then(|value| <[u8; 4]>::try_from(value).ok())
                .ok_or_else(|| io::Error::other("short netlink acknowledgement sequence"))?,
        );
        if kind == 2 && sequence == expected_sequence {
            let code = i32::from_ne_bytes(
                message
                    .get(16..20)
                    .and_then(|value| <[u8; 4]>::try_from(value).ok())
                    .ok_or_else(|| io::Error::other("short netlink acknowledgement"))?,
            );
            return Ok(Some(code));
        }
        remaining = remaining
            .get(length.next_multiple_of(4)..)
            .ok_or_else(|| io::Error::other("invalid netlink acknowledgement padding"))?;
    }
    Ok(None)
}

/// One kernel-selected packet and its connection direction.
#[derive(Debug)]
pub struct Record<'a> {
    pub sequence: u32,
    pub reply: bool,
    pub payload: &'a [u8],
}

/// Validate netlink lengths and required metadata before exposing packet bytes.
pub fn records(mut bytes: &[u8]) -> Result<Vec<Record<'_>>, &'static str> {
    let mut records = Vec::new();
    while !bytes.is_empty() {
        let length = u32::from_ne_bytes(
            bytes.get(..4).ok_or("short netlink header")?.try_into().map_err(|_error| "length")?,
        ) as usize;
        if length < 16 {
            return Err("invalid netlink length");
        }
        let message = bytes.get(..length).ok_or("truncated netlink message")?;
        let kind = u16::from_ne_bytes(
            message.get(4..6).ok_or("type")?.try_into().map_err(|_error| "type")?,
        );
        if kind == 0x400 {
            if message.get(16..20)
                != Some(&[libc::AF_INET as u8, 0, (NFLOG_GROUP >> 8) as u8, NFLOG_GROUP as u8])
            {
                return Err("unexpected NFLOG family/group");
            }
            let mut attributes = message.get(20..).ok_or("short NFLOG header")?;
            let (mut sequence, mut info, mut payload) = (None, None, None);
            while !attributes.is_empty() {
                let length = u16::from_ne_bytes(
                    attributes
                        .get(..2)
                        .ok_or("short attribute")?
                        .try_into()
                        .map_err(|_error| "attribute")?,
                ) as usize;
                let kind = u16::from_ne_bytes(
                    attributes
                        .get(2..4)
                        .ok_or("short attribute")?
                        .try_into()
                        .map_err(|_error| "attribute")?,
                ) & 0x3fff;
                if length < 4 {
                    return Err("invalid attribute length");
                }
                let value = attributes.get(4..length).ok_or("truncated attribute")?;
                match kind {
                    9 => payload = Some(value),
                    12 => {
                        sequence = Some(u32::from_be_bytes(
                            value.try_into().map_err(|_error| "invalid sequence")?,
                        ))
                    }
                    19 => {
                        info = Some(u32::from_be_bytes(
                            value.try_into().map_err(|_error| "invalid conntrack info")?,
                        ))
                    }
                    _ => {}
                }
                attributes =
                    attributes.get(length.next_multiple_of(4)..).ok_or("attribute padding")?;
            }
            let info = info.ok_or("missing conntrack info")?;
            if info > 5 {
                return Err("invalid conntrack direction");
            }
            records.push(Record {
                sequence: sequence.ok_or("missing capture sequence")?,
                reply: info >= 3,
                payload: payload.ok_or("missing packet payload")?,
            });
        } else if kind != 3 {
            return Err("unexpected netlink message type");
        }
        bytes = bytes.get(length.next_multiple_of(4)..).ok_or("netlink padding")?;
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(kind: u16, sequence: u32, body: &[u8]) -> Vec<u8> {
        let length = 16 + body.len();
        let mut message = Vec::with_capacity(length.next_multiple_of(4));
        message.extend(u32::try_from(length).expect("test length").to_ne_bytes());
        message.extend(kind.to_ne_bytes());
        message.extend(0_u16.to_ne_bytes());
        message.extend(sequence.to_ne_bytes());
        message.extend(0_u32.to_ne_bytes());
        message.extend(body);
        message.resize(length.next_multiple_of(4), 0);
        message
    }

    fn attribute(kind: u16, value: &[u8]) -> Vec<u8> {
        let length = 4 + value.len();
        let mut attribute = Vec::with_capacity(length.next_multiple_of(4));
        attribute.extend(u16::try_from(length).expect("attribute length").to_ne_bytes());
        attribute.extend(kind.to_ne_bytes());
        attribute.extend(value);
        attribute.resize(length.next_multiple_of(4), 0);
        attribute
    }

    fn packet_record(info: u32, include_info: bool) -> Vec<u8> {
        let mut body = vec![libc::AF_INET as u8, 0];
        body.extend(NFLOG_GROUP.to_be_bytes());
        body.extend(attribute(12, &9_u32.to_be_bytes()));
        if include_info {
            body.extend(attribute(19, &info.to_be_bytes()));
        }
        body.extend(attribute(9, b"packet"));
        message(0x400, 0, &body)
    }

    #[test]
    fn acknowledgement_skips_interleaved_packet_messages() {
        let mut datagram = message(0x400, 0, &[]);
        datagram.extend(message(2, 7, &0_i32.to_ne_bytes()));

        assert_eq!(acknowledgement(&datagram, 7).expect("valid messages"), Some(0));
    }

    #[test]
    fn acknowledgement_rejects_truncated_messages() {
        let mut datagram = message(2, 7, &0_i32.to_ne_bytes());
        datagram.pop();

        acknowledgement(&datagram, 7).expect_err("truncated message should fail");
    }

    #[test]
    fn records_require_sequence_direction_and_payload() {
        let datagram = packet_record(3, true);
        let parsed = records(&datagram).expect("complete record");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.first().map(|record| record.sequence), Some(9));
        assert!(parsed.first().is_some_and(|record| record.reply));
        assert_eq!(parsed.first().map(|record| record.payload), Some(b"packet".as_slice()));

        assert_eq!(
            records(&packet_record(3, false)).expect_err("direction is required"),
            "missing conntrack info"
        );
        assert_eq!(
            records(&packet_record(6, true)).expect_err("direction must be known"),
            "invalid conntrack direction"
        );
    }

    #[test]
    fn records_reject_every_truncated_message_prefix() {
        let datagram = packet_record(0, true);
        for length in 1..datagram.len() {
            assert!(
                records(datagram.get(..length).expect("test prefix")).is_err(),
                "length {length}"
            );
        }
    }

    #[test]
    fn records_reject_invalid_attribute_lengths_and_wrong_groups() {
        let mut malformed = packet_record(0, true);
        malformed
            .get_mut(20..22)
            .expect("first attribute length")
            .copy_from_slice(&3_u16.to_ne_bytes());
        assert_eq!(
            records(&malformed).expect_err("short attribute must fail"),
            "invalid attribute length"
        );

        let mut wrong_group = packet_record(0, true);
        *wrong_group.get_mut(19).expect("group low byte") ^= 1;
        assert_eq!(
            records(&wrong_group).expect_err("wrong group must fail"),
            "unexpected NFLOG family/group"
        );
    }
}
