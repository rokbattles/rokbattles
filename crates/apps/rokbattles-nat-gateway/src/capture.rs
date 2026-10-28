//! Receive and validate packet copies from Linux NFLOG.
//!
//! [`Capture`] owns a netlink socket bound to the gateway's logging group. Its
//! configuration requests whole IPv4 packets, local sequence numbers, and
//! conntrack direction metadata. [`records`] parses those datagrams without
//! casting unaligned wire bytes to C structures.
//!
//! Netlink headers and attribute headers use native byte order. NFLOG sequence
//! numbers, conntrack metadata, and the group identifier use network byte order.
//! Both messages and attributes advance on four-byte boundaries.
//! Packet messages contain a 16-byte netlink header, a 4-byte nfgenmsg header,
//! and length-prefixed attributes. Attribute lengths include their 4-byte header
//! but exclude alignment padding; see the [NFLOG UAPI].
//!
//! The socket receives copies, not forwarding verdict requests. Losing a copy
//! invalidates userspace cipher state; kernel NAT continues independently.
//!
//! [NFLOG UAPI]: https://github.com/torvalds/linux/blob/v6.8/include/uapi/linux/netfilter/nfnetlink_log.h

use std::{
    io, mem,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};

use crate::rules::NFLOG_GROUP;

/// An owned netlink socket bound to the gateway's exclusive NFLOG group.
///
/// Dropping this value closes the socket but leaves nftables and conntrack state
/// installed. Open it during privileged startup, then retain it after
/// [`crate::service::drop_privileges`].
///
/// The open descriptor does not by itself preserve netfilter write authority.
/// Linux checks `CAP_NET_ADMIN` through [`nfnetlink_rcv()`] and
/// [`netlink_net_capable()`], considering both the socket's credentials and the
/// current sender's capabilities. The startup path verifies that the worker's
/// effective, permitted, inheritable, and ambient capability sets are empty.
///
/// [`nfnetlink_rcv()`]: https://github.com/torvalds/linux/blob/v6.8/net/netfilter/nfnetlink.c
/// [`netlink_net_capable()`]: https://github.com/torvalds/linux/blob/v6.8/net/netlink/af_netlink.c
pub struct Capture {
    socket: OwnedFd,
}

fn os_result(result: i32) -> io::Result<()> {
    if result < 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

impl Capture {
    /// Bind and configure the gateway's NFLOG group.
    ///
    /// Call before dropping capabilities or starting the observer. Requests a
    /// 4 MiB receive buffer; Linux may report twice the requested size. If the
    /// privileged buffer override fails, falls back to the ordinary socket limit.
    /// Configuration waits for matching kernel acknowledgements and can block.
    ///
    /// # Errors
    ///
    /// Returns socket, bind, buffer-configuration, or netfilter errors, including
    /// lack of `CAP_NET_ADMIN` or another listener owning the group. Also rejects
    /// malformed acknowledgements and failure to find an acknowledgement within
    /// 32 received datagrams. This datagram limit is not a wall-clock timeout.
    pub fn open() -> io::Result<Self> {
        // SAFETY: socket receives only integer arguments; success yields a new descriptor.
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                libc::NETLINK_NETFILTER,
            )
        };
        os_result(fd)?;
        // SAFETY: os_result rejected negative descriptors. This successful socket
        // result is open and has no owner; OwnedFd takes sole responsibility for closing it.
        let socket = unsafe { OwnedFd::from_raw_fd(fd) };

        // SAFETY: sockaddr_nl contains integer fields and accepts an all-zero bit pattern.
        let mut address: libc::sockaddr_nl = unsafe { mem::zeroed() };
        address.nl_family = libc::AF_NETLINK as u16;
        // SAFETY: fd is open. address is initialized and correctly aligned, and
        // bind reads at most the supplied sockaddr_nl size during this call.
        os_result(unsafe {
            libc::bind(fd, (&raw const address).cast(), mem::size_of_val(&address) as u32)
        })?;

        let size: i32 = 4 * 1024 * 1024;
        // SO_RCVBUF is capped by rmem_max. Use the startup capability to request
        // a larger buffer for this socket without changing a global sysctl.
        // SAFETY: fd is open; size is an initialized, aligned integer, and optlen
        // matches its allocation. setsockopt does not retain the pointer.
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
            // FORCE may be unavailable in a user namespace. The fallback still
            // provides capture, with the host's ordinary receive-buffer ceiling.
            // SAFETY: the same live descriptor, integer pointer, and exact length
            // used above remain valid for this synchronous socket-option call.
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
        // SAFETY: both outputs are initialized and aligned. option_len starts
        // at actual's allocation size, bounding the kernel's write to actual.
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
        // NFULA_CFG_CMD binds this group; NFULA_CFG_MODE requests complete
        // layer-3 copies; NFULA_CFG_FLAGS enables local sequence and conntrack info.
        capture.configure(1, &[1], 1)?;
        let mut mode = 65535_u32.to_be_bytes().to_vec();
        mode.extend([2, 0]); // NFULNL_COPY_PACKET plus the reserved byte.
        capture.configure(2, &mode, 2)?;
        capture.configure(6, &5_u16.to_be_bytes(), 3)?;
        Ok(capture)
    }

    /// Send one NFULNL_MSG_CONFIG attribute and await its sequence-matched ACK.
    /// Packet datagrams encountered during startup are deliberately not decoded.
    fn configure(&self, kind: u16, value: &[u8], sequence: u32) -> io::Result<()> {
        let attr_len = 4 + value.len();
        let length = 20 + attr_len.next_multiple_of(4);
        let mut message = Vec::with_capacity(length);
        message.extend((length as u32).to_ne_bytes());
        message.extend(0x401_u16.to_ne_bytes());
        message.extend(5_u16.to_ne_bytes()); // NLM_F_REQUEST | NLM_F_ACK.
        message.extend(sequence.to_ne_bytes());
        message.extend(0_u32.to_ne_bytes());
        message.extend([libc::AF_INET as u8, 0]);
        message.extend(NFLOG_GROUP.to_be_bytes());
        message.extend((attr_len as u16).to_ne_bytes());
        message.extend(kind.to_ne_bytes());
        message.extend(value);
        message.resize(length, 0);
        // SAFETY: sockaddr_nl has no invalid all-zero field values; padding is initialized too.
        let mut kernel: libc::sockaddr_nl = unsafe { mem::zeroed() };
        kernel.nl_family = libc::AF_NETLINK as u16;
        // SAFETY: the descriptor is owned by self. message is readable for its
        // full length; kernel is an initialized, aligned sockaddr_nl with the
        // matching length. sendto borrows both allocations only for this call.
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

    /// Block until one complete kernel datagram is copied into `buffer`.
    ///
    /// Returns the number of initialized bytes to pass to [`records`]. The source
    /// netlink port ID must be zero, identifying the kernel. `MSG_TRUNC` requests
    /// the original datagram length so an undersized buffer cannot silently
    /// produce a valid-looking prefix.
    ///
    /// # Errors
    ///
    /// Returns receive errors, including `EINTR` and `ENOBUFS`, or rejects a
    /// userspace source or truncated datagram. `EINTR` may be retried; `ENOBUFS`
    /// means decoder state must be discarded. The binary exits on other errors.
    pub fn receive(&self, buffer: &mut [u8]) -> io::Result<usize> {
        // SAFETY: every sockaddr_nl field permits zero; recvfrom receives initialized storage.
        let mut source: libc::sockaddr_nl = unsafe { mem::zeroed() };
        let mut length = mem::size_of_val(&source) as libc::socklen_t;
        // SAFETY: buffer is exclusively borrowed and writable for buffer.len()
        // bytes. source and length are aligned, initialized outputs; length
        // bounds the address write. recvfrom retains none of these pointers.
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

/// Find an NLMSG_ERROR acknowledgement for one configuration request.
/// Its signed error code is native-endian: zero acknowledges success, and a
/// negative value encodes errno. Other messages and request sequences are skipped.
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

/// A packet copy with the metadata needed to reconstruct its server stream.
#[derive(Debug)]
pub struct Record<'a> {
    /// Per-group capture sequence, wrapping independently of TCP sequence numbers.
    pub sequence: u32,
    /// Whether conntrack classifies the packet in the connection's reply direction.
    pub reply: bool,
    /// Borrowed layer-3 packet bytes; these have not yet passed [`crate::packet::parse`].
    pub payload: &'a [u8],
}

/// Parse a complete netlink datagram into borrowed NFLOG packet records.
///
/// Accepts packet messages for this IPv4 group and ignores NLMSG_DONE markers.
/// Unknown attributes are skipped after their lengths are checked. Sequence,
/// conntrack info, and payload attributes are required; later duplicate attributes
/// replace earlier values. An empty datagram produces an empty vector.
///
/// # Errors
///
/// Returns a reason for malformed message or attribute framing, missing padding,
/// a wrong family/group, absent metadata, or unsupported message/direction values.
/// Validation is all-or-nothing: an invalid later message discards earlier records
/// in the same datagram. The caller must retire observer state after this error.
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
                // The two high bits are netlink attribute flags, not the type.
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
                // IP_CT_IS_REPLY is the offset separating original and reply states.
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

    #[test]
    fn acknowledgements_match_sequence_and_preserve_kernel_error_codes() {
        let mut bytes = message(2, 6, &0_i32.to_ne_bytes());
        bytes.extend(message(3, 0, &[]));
        bytes.extend(message(2, 7, &(-libc::EPERM).to_ne_bytes()));

        assert_eq!(acknowledgement(&bytes, 7).expect("matching ACK"), Some(-libc::EPERM));
        assert_eq!(acknowledgement(&bytes, 8).expect("unrelated requests"), None);
    }

    #[test]
    fn acknowledgements_reject_invalid_headers_and_missing_alignment_padding() {
        acknowledgement(&[0; 15], 1).expect_err("short header");
        acknowledgement(&message(3, 0, &[])[..15], 1).expect_err("truncated header");

        let mut bytes = message(3, 0, &[1]);
        bytes.pop();
        acknowledgement(&bytes, 1).expect_err("missing padding");
    }

    #[test]
    fn records_accept_aligned_unknown_attributes_and_both_directions() {
        for info in 0_u32..=4 {
            let mut body = vec![libc::AF_INET as u8, 0];
            body.extend(NFLOG_GROUP.to_be_bytes());
            body.extend(attribute(12, &u32::MAX.to_be_bytes()));
            body.extend(attribute(0x8000 | 123, b"x"));
            body.extend(attribute(19, &info.to_be_bytes()));
            body.extend(attribute(9, b"old"));
            body.extend(attribute(9, b"new"));
            let mut bytes = message(3, 0, &[]);
            bytes.extend(message(0x400, 0, &body));
            bytes.extend(message(3, 0, &[]));

            let parsed = records(&bytes).expect("complete aligned datagram");
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0].sequence, u32::MAX);
            assert_eq!(parsed[0].reply, info >= 3);
            assert_eq!(parsed[0].payload, b"new", "last duplicate attribute wins");
        }
    }

    #[test]
    fn invalid_later_messages_reject_the_whole_datagram() {
        let mut bytes = packet_record(0, true);
        bytes.extend(message(2, 0, &0_i32.to_ne_bytes()));

        assert_eq!(
            records(&bytes).expect_err("no partial records"),
            "unexpected netlink message type"
        );
        assert!(records(&[]).expect("empty datagram").is_empty());
    }

    #[test]
    fn every_required_packet_attribute_is_checked_independently() {
        for (missing, reason) in [
            (12, "missing capture sequence"),
            (19, "missing conntrack info"),
            (9, "missing packet payload"),
        ] {
            let mut body = vec![libc::AF_INET as u8, 0];
            body.extend(NFLOG_GROUP.to_be_bytes());
            for (kind, value) in [
                (12, 7_u32.to_be_bytes().to_vec()),
                (19, 0_u32.to_be_bytes().to_vec()),
                (9, b"packet".to_vec()),
            ] {
                if kind != missing {
                    body.extend(attribute(kind, &value));
                }
            }
            assert_eq!(records(&message(0x400, 0, &body)).expect_err("required attribute"), reason);
        }
    }
}
