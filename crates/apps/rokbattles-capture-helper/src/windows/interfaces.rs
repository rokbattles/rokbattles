//! Bounded OS-only Npcap interface discovery. No request, environment variable,
//! registry string, friendly name, or user-provided address selects a device.

use std::{
    collections::BTreeSet,
    fmt, io,
    mem::{align_of, offset_of, size_of},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};
use windows_sys::Win32::{
    NetworkManagement::{
        IpHelper::{
            IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
        },
        Ndis::IfOperStatusUp,
    },
    Networking::WinSock::{
        AF_INET, AF_INET6, IpDadStateDeprecated, IpDadStatePreferred, SOCKADDR_IN, SOCKADDR_IN6,
        SOCKET_ADDRESS,
    },
};

const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ADAPTERS: usize = 64;
const MAX_ADDRESSES: usize = 128;

pub struct Interface {
    pub name: String,
    pub addresses: Vec<IpAddr>,
}
impl fmt::Debug for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Interface(<redacted>)")
    }
}
fn unavailable() -> io::Error {
    io::Error::other("capture interface discovery unavailable")
}

/// All pointers returned by GetAdaptersAddresses must refer to its one owned
/// allocation. Derive dereference pointers from that allocation after checking
/// addresses and extents; never dereference a returned pointer directly.
struct Buffer<'a>(&'a [u8]);
impl<'a> Buffer<'a> {
    fn range(&self, address: *const u8, length: usize) -> io::Result<&'a [u8]> {
        let start = address.addr().checked_sub(self.0.as_ptr().addr()).ok_or_else(unavailable)?;
        let end = start.checked_add(length).ok_or_else(unavailable)?;
        self.0.get(start..end).ok_or_else(unavailable)
    }
    /// # Safety
    /// T must be an OS C-layout structure containing only integer/pointer/union
    /// fields for which every bit pattern is valid. Its first DWORD is Length.
    unsafe fn node<T: Copy>(&self, address: *const T) -> io::Result<T> {
        if !address.addr().is_multiple_of(align_of::<T>()) {
            return Err(unavailable());
        }
        let header = self.range(address.cast(), size_of::<u32>())?;
        let length =
            u32::from_ne_bytes(header.try_into().map_err(|_error| unavailable())?) as usize;
        if length < size_of::<T>() {
            return Err(unavailable());
        }
        let bytes = self.range(address.cast(), length)?;
        // SAFETY: checked full declared and T extents in the owned allocation;
        // the caller guarantees all-bit-pattern-valid OS integer/pointer fields.
        Ok(unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<T>()) })
    }
    fn device_name(&self, address: *const u8) -> io::Result<String> {
        // AdapterName is the immutable OS adapter identifier, not FriendlyName.
        let bytes = self.range(address, 39)?;
        if bytes.last() != Some(&0) {
            return Err(unavailable());
        }
        let guid = bytes.get(..38).ok_or_else(unavailable)?;
        if guid.first() != Some(&b'{') || guid.last() != Some(&b'}') {
            return Err(unavailable());
        }
        for (index, byte) in guid.iter().enumerate().skip(1).take(36) {
            if matches!(index, 9 | 14 | 19 | 24) {
                if *byte != b'-' {
                    return Err(unavailable());
                }
            } else if !byte.is_ascii_hexdigit() {
                return Err(unavailable());
            }
        }
        let guid = std::str::from_utf8(guid).map_err(|_error| unavailable())?;
        Ok(format!(r"\Device\NPF_{}", guid.to_ascii_uppercase()))
    }
    fn ip(&self, address: SOCKET_ADDRESS) -> io::Result<Option<IpAddr>> {
        let length = usize::try_from(address.iSockaddrLength).map_err(|_error| unavailable())?;
        if !(2..=128).contains(&length) {
            return Err(unavailable());
        }
        let bytes = self.range(address.lpSockaddr.cast(), length)?;
        let family = u16::from_ne_bytes(
            bytes.get(..2).ok_or_else(unavailable)?.try_into().map_err(|_error| unavailable())?,
        );
        let ip = match family {
            AF_INET => {
                if length != size_of::<SOCKADDR_IN>() {
                    return Err(unavailable());
                }
                let offset = offset_of!(SOCKADDR_IN, sin_addr);
                let address =
                    <[u8; 4]>::try_from(bytes.get(offset..offset + 4).ok_or_else(unavailable)?)
                        .map_err(|_error| unavailable())?;
                IpAddr::V4(Ipv4Addr::from(address))
            }
            AF_INET6 => {
                if length != size_of::<SOCKADDR_IN6>() {
                    return Err(unavailable());
                }
                let scope = offset_of!(SOCKADDR_IN6, Anonymous);
                let flow = offset_of!(SOCKADDR_IN6, sin6_flowinfo);
                if bytes
                    .get(scope..scope + 4)
                    .ok_or_else(unavailable)?
                    .iter()
                    .any(|byte| *byte != 0)
                    || bytes
                        .get(flow..flow + 4)
                        .ok_or_else(unavailable)?
                        .iter()
                        .any(|byte| *byte != 0)
                {
                    return Ok(None);
                }
                let offset = offset_of!(SOCKADDR_IN6, sin6_addr);
                let address =
                    <[u8; 16]>::try_from(bytes.get(offset..offset + 16).ok_or_else(unavailable)?)
                        .map_err(|_error| unavailable())?;
                IpAddr::V6(Ipv6Addr::from(address))
            }
            _ => return Err(unavailable()),
        };
        let usable = match ip {
            IpAddr::V4(ip) => {
                !ip.is_unspecified()
                    && !ip.is_multicast()
                    && !ip.is_broadcast()
                    && !ip.is_loopback()
            }
            IpAddr::V6(ip) => {
                !ip.is_unspecified()
                    && !ip.is_multicast()
                    && !ip.is_loopback()
                    && !ip.is_unicast_link_local()
            }
        };
        Ok(usable.then_some(ip))
    }
}

fn decode(bytes: &[u8]) -> io::Result<Vec<Interface>> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(unavailable());
    }
    let buffer = Buffer(bytes);
    let mut result = Vec::new();
    let mut seen_adapters = BTreeSet::new();
    let mut seen_unicast = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut current = bytes.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !current.is_null() {
        if seen_adapters.len() >= MAX_ADAPTERS || !seen_adapters.insert(current.addr()) {
            return Err(unavailable());
        }
        // SAFETY: official C layout consists only of integers, pointers and
        // all-bit-pattern-valid unions; Buffer checks Length and full extent.
        let adapter = unsafe { buffer.node(current) }?;
        let active =
            adapter.OperStatus == IfOperStatusUp && adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK;
        let name = if active { Some(buffer.device_name(adapter.AdapterName)?) } else { None };
        let mut addresses = BTreeSet::new();
        let mut unicast = adapter.FirstUnicastAddress;
        while !unicast.is_null() {
            // One global limit/visited set also rejects lists shared by adapters.
            if seen_unicast.len() >= MAX_ADDRESSES || !seen_unicast.insert(unicast.addr()) {
                return Err(unavailable());
            }
            // SAFETY: same all-integer/pointer C-layout contract and checked node.
            let entry = unsafe { buffer.node::<IP_ADAPTER_UNICAST_ADDRESS_LH>(unicast) }?;
            let ip = buffer.ip(entry.Address)?;
            if active
                && entry.ValidLifetime != 0
                && (entry.DadState == IpDadStatePreferred || entry.DadState == IpDadStateDeprecated)
                && let Some(ip) = ip
            {
                addresses.insert(ip);
            }
            unicast = entry.Next;
        }
        if !addresses.is_empty()
            && let Some(name) = name
        {
            if !names.insert(name.clone()) {
                return Err(unavailable());
            }
            result.push(Interface { name, addresses: addresses.into_iter().collect() });
        }
        current = adapter.Next;
    }
    Ok(result)
}

/// Read once during capture setup. This never installs/starts a driver, opens a
/// capture device, resolves user-controlled paths, or changes adapter settings.
#[cfg(windows)]
pub fn enumerate() -> io::Result<Vec<Interface>> {
    use windows_sys::Win32::{
        Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA},
        NetworkManagement::IpHelper::{
            GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_FRIENDLY_NAME,
            GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
        },
        Networking::WinSock::AF_UNSPEC,
    };
    const FLAGS: u32 = GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER
        | GAA_FLAG_SKIP_FRIENDLY_NAME;
    // Microsoft recommends an initial 15KB allocation to avoid a size-only call.
    let mut needed = 15_000u32;
    for _ in 0..3 {
        if needed as usize > MAX_BYTES || (needed as usize) < size_of::<IP_ADAPTER_ADDRESSES_LH>() {
            return Err(unavailable());
        }
        let capacity = needed as usize;
        let mut words = vec![0u64; capacity.div_ceil(size_of::<u64>())];
        // SAFETY: aligned, initialized owned allocation at least needed bytes;
        // bounded writable size DWORD, supported family/flags and null Reserved.
        let status = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_UNSPEC),
                FLAGS,
                ptr::null(),
                words.as_mut_ptr().cast(),
                &mut needed,
            )
        };
        if status == ERROR_NO_DATA {
            return Ok(Vec::new());
        }
        if status == 0 {
            if needed as usize > capacity {
                return Err(unavailable());
            }
            // SAFETY: output length was checked against the initialized allocation.
            let bytes =
                unsafe { std::slice::from_raw_parts(words.as_ptr().cast::<u8>(), needed as usize) };
            return decode(bytes);
        }
        if status != ERROR_BUFFER_OVERFLOW || needed as usize <= capacity {
            return Err(unavailable());
        }
    }
    Err(unavailable())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ADAPTER: usize = 0;
    const NAME: usize = 1024;
    const UNICAST: usize = 2048;
    const SOCKET: usize = 3072;
    const GUID: &str = "{01234567-89ab-cdef-0123-456789abcdef}";
    struct Fixture(Vec<u64>);
    impl Fixture {
        fn new() -> Self {
            Self(vec![0u64; 8192])
        }
        fn bytes(&self) -> &[u8] {
            // SAFETY: u64 allocation is fully initialized; all fields are written
            // as bytes so no struct copy can introduce uninitialized padding.
            unsafe {
                std::slice::from_raw_parts(self.0.as_ptr().cast(), self.0.len() * size_of::<u64>())
            }
        }
        fn write(&mut self, offset: usize, bytes: &[u8]) {
            let length = self.0.len() * size_of::<u64>();
            // SAFETY: unique fully initialized u64 allocation; arbitrary byte
            // writes preserve u64 validity and the allocation never moves.
            let storage =
                unsafe { std::slice::from_raw_parts_mut(self.0.as_mut_ptr().cast::<u8>(), length) };
            storage
                .get_mut(offset..offset + bytes.len())
                .expect("fixture extent")
                .copy_from_slice(bytes);
        }
        fn pointer(&mut self, offset: usize, target: Option<usize>) {
            let address = target.map_or(0, |offset| self.0.as_ptr().addr() + offset);
            self.write(offset, &address.to_ne_bytes());
        }
        fn adapter(
            &mut self,
            offset: usize,
            next: Option<usize>,
            unicast: Option<usize>,
            name: usize,
        ) {
            self.write(offset, &(size_of::<IP_ADAPTER_ADDRESSES_LH>() as u32).to_ne_bytes());
            self.pointer(offset + offset_of!(IP_ADAPTER_ADDRESSES_LH, Next), next);
            self.pointer(offset + offset_of!(IP_ADAPTER_ADDRESSES_LH, AdapterName), Some(name));
            self.pointer(
                offset + offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstUnicastAddress),
                unicast,
            );
            self.write(
                offset + offset_of!(IP_ADAPTER_ADDRESSES_LH, OperStatus),
                &IfOperStatusUp.to_ne_bytes(),
            );
            self.write(offset + offset_of!(IP_ADAPTER_ADDRESSES_LH, IfType), &6u32.to_ne_bytes());
            self.write(name, GUID.as_bytes());
            self.write(name + 38, &[0]);
        }
        fn unicast(&mut self, offset: usize, next: Option<usize>, socket: usize, ipv6: bool) {
            self.write(offset, &(size_of::<IP_ADAPTER_UNICAST_ADDRESS_LH>() as u32).to_ne_bytes());
            self.pointer(offset + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Next), next);
            let address = offset + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Address);
            self.pointer(address + offset_of!(SOCKET_ADDRESS, lpSockaddr), Some(socket));
            let length = if ipv6 { size_of::<SOCKADDR_IN6>() } else { size_of::<SOCKADDR_IN>() };
            self.write(
                address + offset_of!(SOCKET_ADDRESS, iSockaddrLength),
                &(length as i32).to_ne_bytes(),
            );
            self.write(
                offset + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, DadState),
                &IpDadStatePreferred.to_ne_bytes(),
            );
            self.write(
                offset + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, ValidLifetime),
                &3600u32.to_ne_bytes(),
            );
            self.write(socket, &(if ipv6 { AF_INET6 } else { AF_INET }).to_ne_bytes());
            if ipv6 {
                self.write(
                    socket + offset_of!(SOCKADDR_IN6, sin6_addr),
                    &"2001:db8::2".parse::<Ipv6Addr>().expect("v6").octets(),
                );
            } else {
                self.write(socket + offset_of!(SOCKADDR_IN, sin_addr), &[192, 0, 2, 2]);
            }
        }
        fn one() -> Self {
            let mut fixture = Self::new();
            fixture.adapter(ADAPTER, None, Some(UNICAST), NAME);
            fixture.unicast(UNICAST, None, SOCKET, false);
            fixture
        }
    }

    #[test]
    fn returns_only_fixed_os_guid_device_with_ipv4_and_ipv6_addresses() {
        let mut fixture = Fixture::one();
        fixture.unicast(UNICAST, Some(UNICAST + 128), SOCKET, false);
        fixture.unicast(UNICAST + 128, None, SOCKET + 128, true);
        let result = decode(fixture.bytes()).expect("OS fixture");
        assert_eq!(result.len(), 1);
        let interface = result.first().expect("interface");
        assert_eq!(interface.name, r"\Device\NPF_{01234567-89AB-CDEF-0123-456789ABCDEF}");
        assert_eq!(
            interface.addresses,
            vec![
                "192.0.2.2".parse::<IpAddr>().expect("v4"),
                "2001:db8::2".parse::<IpAddr>().expect("v6")
            ]
        );
        let debug = format!("{interface:?}");
        assert!(!debug.contains("192.0.2") && !debug.contains("01234567"));
    }

    #[test]
    fn inactive_loopback_tentative_and_expired_interfaces_are_not_selected() {
        for case in 0..4 {
            let mut fixture = Fixture::one();
            match case {
                0 => fixture
                    .write(offset_of!(IP_ADAPTER_ADDRESSES_LH, OperStatus), &2i32.to_ne_bytes()),
                1 => fixture.write(
                    offset_of!(IP_ADAPTER_ADDRESSES_LH, IfType),
                    &IF_TYPE_SOFTWARE_LOOPBACK.to_ne_bytes(),
                ),
                2 => fixture.write(
                    UNICAST + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, DadState),
                    &1i32.to_ne_bytes(),
                ),
                _ => fixture.write(
                    UNICAST + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, ValidLifetime),
                    &0u32.to_ne_bytes(),
                ),
            }
            assert!(decode(fixture.bytes()).expect("safe empty selection").is_empty());
        }
    }

    #[test]
    fn invalid_guid_or_path_injection_cannot_become_a_pcap_device() {
        for bad in [&b"/"[..], &b"\0"[..], &b"z"[..]] {
            let mut fixture = Fixture::one();
            fixture.write(NAME + 1, bad);
            decode(fixture.bytes()).expect_err("reject name");
        }
        let mut fixture = Fixture::one();
        fixture.write(NAME + 38, b"x");
        decode(fixture.bytes()).expect_err("require terminator");
    }

    #[test]
    fn out_of_allocation_pointers_and_short_or_oversized_nodes_fail_closed() {
        for field in [
            offset_of!(IP_ADAPTER_ADDRESSES_LH, Next),
            offset_of!(IP_ADAPTER_ADDRESSES_LH, AdapterName),
            offset_of!(IP_ADAPTER_ADDRESSES_LH, FirstUnicastAddress),
        ] {
            let mut fixture = Fixture::one();
            fixture.pointer(field, Some(fixture.bytes().len() + 64));
            decode(fixture.bytes()).expect_err("outside owned output");
        }
        for length in [0u32, 4, u32::MAX] {
            let mut fixture = Fixture::one();
            fixture.write(ADAPTER, &length.to_ne_bytes());
            decode(fixture.bytes()).expect_err("adapter length");
            let mut fixture = Fixture::one();
            fixture.write(UNICAST, &length.to_ne_bytes());
            decode(fixture.bytes()).expect_err("unicast length");
        }
        let fixture = Fixture::one();
        decode(fixture.bytes().get(..SOCKET + 8).expect("prefix")).expect_err("socket extent");
    }

    #[test]
    fn cyclic_or_shared_linked_lists_fail_closed() {
        let mut fixture = Fixture::one();
        fixture.pointer(offset_of!(IP_ADAPTER_ADDRESSES_LH, Next), Some(ADAPTER));
        decode(fixture.bytes()).expect_err("adapter cycle");
        let mut fixture = Fixture::one();
        fixture.pointer(UNICAST + offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Next), Some(UNICAST));
        decode(fixture.bytes()).expect_err("unicast cycle");
        let mut fixture = Fixture::one();
        fixture.adapter(ADAPTER, Some(512), Some(UNICAST), NAME);
        fixture.adapter(512, None, Some(UNICAST), NAME + 64);
        decode(fixture.bytes()).expect_err("shared unicast node");
    }

    #[test]
    fn scoped_nonunicast_and_loopback_addresses_are_excluded() {
        for address in [
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::BROADCAST,
            Ipv4Addr::new(224, 0, 0, 1),
        ] {
            let mut fixture = Fixture::one();
            fixture.write(SOCKET + offset_of!(SOCKADDR_IN, sin_addr), &address.octets());
            assert!(decode(fixture.bytes()).expect("v4 filter").is_empty());
        }
        for address in [
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::LOCALHOST,
            "fe80::1".parse().expect("link local"),
            "ff02::1".parse().expect("multicast"),
        ] {
            let mut fixture = Fixture::one();
            fixture.unicast(UNICAST, None, SOCKET, true);
            fixture.write(SOCKET + offset_of!(SOCKADDR_IN6, sin6_addr), &address.octets());
            assert!(decode(fixture.bytes()).expect("v6 filter").is_empty());
        }
        let mut fixture = Fixture::one();
        fixture.unicast(UNICAST, None, SOCKET, true);
        fixture.write(SOCKET + offset_of!(SOCKADDR_IN6, Anonymous), &1u32.to_ne_bytes());
        assert!(decode(fixture.bytes()).expect("scope filter").is_empty());
    }

    #[test]
    fn adapter_and_address_counts_are_bounded() {
        let mut fixture = Fixture::new();
        for index in 0..=MAX_ADAPTERS {
            fixture.adapter(
                index * 512,
                (index < MAX_ADAPTERS).then_some((index + 1) * 512),
                None,
                40_000,
            );
        }
        decode(fixture.bytes()).expect_err("adapter count");
        let mut fixture = Fixture::one();
        for index in 0..=MAX_ADDRESSES {
            fixture.unicast(
                UNICAST + index * 128,
                (index < MAX_ADDRESSES).then_some(UNICAST + (index + 1) * 128),
                40_000,
                false,
            );
        }
        decode(fixture.bytes()).expect_err("address count");
    }
}
