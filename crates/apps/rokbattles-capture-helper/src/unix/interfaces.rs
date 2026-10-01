//! Enumerate local interfaces through the OS; callers cannot supply names/addresses.
use std::{
    collections::BTreeMap,
    ffi::CStr,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr::{self, NonNull},
};

const MAX_INTERFACES: usize = 32;
const MAX_ADDRESSES: usize = 16;
const MAX_ROWS: usize = 1024;

#[derive(Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub addresses: Vec<IpAddr>,
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "local interface scope unavailable")
}

pub fn enumerate() -> io::Result<Vec<Interface>> {
    let mut head: *mut libc::ifaddrs = ptr::null_mut();
    // SAFETY: writable output; successful allocation is owned by Allocation below.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(io::Error::last_os_error());
    }
    struct Allocation(Option<NonNull<libc::ifaddrs>>);
    impl Drop for Allocation {
        fn drop(&mut self) {
            if let Some(head) = self.0 {
                // SAFETY: exact successful getifaddrs allocation, freed once after traversal.
                unsafe { libc::freeifaddrs(head.as_ptr()) };
            }
        }
    }
    let allocation = Allocation(NonNull::new(head));
    // SAFETY: getifaddrs succeeded, so every linked row and its fields meet the
    // OS allocation contract below. The guard keeps the allocation alive until
    // traversal returns (including errors); no pointer or borrow escapes it.
    unsafe { enumerate_rows(allocation.0) }
}

/// Traverse an already-owned native interface list without retaining its pointers.
///
/// # Safety
/// Every reachable non-null node must point to an initialized, aligned `ifaddrs`.
/// Non-null names must point to live NUL-terminated byte strings. Non-null address
/// pointers must be aligned for `sockaddr` and its family-specific representation,
/// with initialized `sockaddr_in`/`sockaddr_in6` storage for AF_INET/AF_INET6.
/// The caller must keep all nodes, names and addresses alive and immutable for
/// the entire call. Production obtains this contract from a successful
/// `getifaddrs`; synthetic callers must own equivalent backing storage. Null
/// fields are accepted. Cycles are bounded by MAX_ROWS and return an error.
unsafe fn enumerate_rows(head: Option<NonNull<libc::ifaddrs>>) -> io::Result<Vec<Interface>> {
    let mut interfaces = BTreeMap::<String, Vec<IpAddr>>::new();
    let mut cursor = head;
    let mut count = 0;
    while let Some(pointer) = cursor {
        count += 1;
        if count > MAX_ROWS {
            return Err(invalid());
        }
        // SAFETY: the caller guarantees this non-null node is aligned and live.
        let row = unsafe { pointer.as_ref() };
        cursor = NonNull::new(row.ifa_next);
        if row.ifa_flags & libc::IFF_UP as u32 == 0
            || row.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
        {
            continue;
        }
        let (Some(address), Some(name)) = (NonNull::new(row.ifa_addr), NonNull::new(row.ifa_name))
        else {
            continue;
        };
        // SAFETY: the caller guarantees this non-null name is live and NUL-terminated.
        let name = unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().map_err(|_error| invalid())?;
        if name.is_empty() || name.len() >= libc::IFNAMSIZ || name.contains("://") {
            return Err(invalid());
        }
        // SAFETY: the caller guarantees this non-null sockaddr is initialized and aligned.
        let family = unsafe { address.as_ref() }.sa_family as i32;
        let address = match family {
            libc::AF_INET => {
                // SAFETY: AF_INET identifies the native sockaddr_in layout.
                let address = unsafe { address.cast::<libc::sockaddr_in>().as_ref() };
                IpAddr::V4(Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes()))
            }
            libc::AF_INET6 => {
                // SAFETY: AF_INET6 identifies the native sockaddr_in6 layout.
                let address = unsafe { address.cast::<libc::sockaddr_in6>().as_ref() };
                // Scoped addresses cannot be represented unambiguously by packet IPC.
                if address.sin6_scope_id != 0 {
                    continue;
                }
                IpAddr::V6(Ipv6Addr::from(address.sin6_addr.s6_addr))
            }
            _ => continue,
        };
        if !admissible(address) {
            continue;
        }
        let addresses = interfaces.entry(name.to_owned()).or_default();
        if !addresses.contains(&address) {
            addresses.push(address);
        }
        if addresses.len() > MAX_ADDRESSES || interfaces.len() > MAX_INTERFACES {
            return Err(invalid());
        }
    }
    if interfaces.is_empty() {
        return Err(invalid());
    }
    Ok(interfaces
        .into_iter()
        .map(|(name, mut addresses)| {
            addresses.sort();
            Interface { name, addresses }
        })
        .collect())
}
fn admissible(address: IpAddr) -> bool {
    !address.is_unspecified()
        && !address.is_multicast()
        && !address.is_loopback()
        && match address {
            IpAddr::V4(address) => !address.is_broadcast(),
            IpAddr::V6(address) => !address.is_unicast_link_local(),
        }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    enum Address {
        V4(Box<libc::sockaddr_in>),
        V6(Box<libc::sockaddr_in6>),
        Other(Box<libc::sockaddr>),
    }
    impl Address {
        fn ip(value: &str) -> Self {
            match value.parse::<IpAddr>().expect("fixture address") {
                IpAddr::V4(value) => {
                    // SAFETY: all fields are integers/byte arrays; zero is valid.
                    let mut address: libc::sockaddr_in = unsafe { std::mem::zeroed() };
                    address.sin_family = libc::AF_INET as _;
                    address.sin_addr.s_addr = u32::from_ne_bytes(value.octets());
                    Self::V4(Box::new(address))
                }
                IpAddr::V6(value) => {
                    // SAFETY: all fields are integers/byte arrays; zero is valid.
                    let mut address: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
                    address.sin6_family = libc::AF_INET6 as _;
                    address.sin6_addr.s6_addr = value.octets();
                    Self::V6(Box::new(address))
                }
            }
        }
        fn unsupported() -> Self {
            // SAFETY: all fields are integers/byte arrays; zero is valid.
            let mut address: libc::sockaddr = unsafe { std::mem::zeroed() };
            address.sa_family = libc::AF_UNSPEC as _;
            Self::Other(Box::new(address))
        }
        fn pointer(&mut self) -> *mut libc::sockaddr {
            match self {
                Self::V4(address) => ptr::from_mut(address.as_mut()).cast(),
                Self::V6(address) => ptr::from_mut(address.as_mut()).cast(),
                Self::Other(address) => ptr::from_mut(address.as_mut()),
            }
        }
    }

    struct Row {
        native: Box<libc::ifaddrs>,
        // Keep every native pointer's backing allocation alive until after traversal.
        _name: Option<CString>,
        _address: Option<Address>,
    }
    impl Row {
        fn new(name: Option<&[u8]>, flags: u32, mut address: Option<Address>) -> Self {
            let name = name.map(|value| CString::new(value).expect("fixture name"));
            // SAFETY: integer flags and nullable raw pointers all admit zero.
            let mut native: libc::ifaddrs = unsafe { std::mem::zeroed() };
            native.ifa_name =
                name.as_ref().map_or(ptr::null_mut(), |value| value.as_ptr().cast_mut());
            native.ifa_addr = address.as_mut().map_or(ptr::null_mut(), Address::pointer);
            native.ifa_flags = flags;
            Self { native: Box::new(native), _name: name, _address: address }
        }
        fn up(name: &str, address: &str) -> Self {
            Self::new(Some(name.as_bytes()), libc::IFF_UP as u32, Some(Address::ip(address)))
        }
    }

    struct List(Vec<Row>);
    impl List {
        fn new(mut rows: Vec<Row>) -> Self {
            let mut next = ptr::null_mut();
            for row in rows.iter_mut().rev() {
                row.native.ifa_next = next;
                next = ptr::from_mut(row.native.as_mut());
            }
            Self(rows)
        }
        fn read(&self) -> io::Result<Vec<Interface>> {
            let head = self.0.first().map(|row| NonNull::from(row.native.as_ref()));
            // SAFETY: boxed nodes/addresses and CString names remain owned and
            // immutably borrowed for the call. Every link is null or points to
            // another live boxed node, and each address matches its family.
            unsafe { enumerate_rows(head) }
        }
        fn cycle(&mut self) {
            let first = self.0.first_mut().map(|row| ptr::from_mut(row.native.as_mut()));
            if let (Some(first), Some(last)) = (first, self.0.last_mut()) {
                last.native.ifa_next = first;
            }
        }
    }

    fn assert_invalid(list: &List) {
        let error = list.read().err().expect("invalid fixture must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn null_head_and_a_list_without_usable_addresses_fail_closed() {
        assert_invalid(&List::new(vec![]));
        assert_invalid(&List::new(vec![Row::new(None, libc::IFF_UP as u32, None)]));
    }

    #[test]
    fn null_fields_inactive_loopback_and_unsupported_rows_are_skipped() {
        let list = List::new(vec![
            Row::new(None, libc::IFF_UP as u32, Some(Address::ip("192.0.2.1"))),
            Row::new(Some(b"missing"), libc::IFF_UP as u32, None),
            Row::new(Some(b"down"), 0, Some(Address::ip("192.0.2.1"))),
            Row::new(
                Some(b"loop"),
                (libc::IFF_UP | libc::IFF_LOOPBACK) as u32,
                Some(Address::ip("192.0.2.1")),
            ),
            Row::new(Some(b"other"), libc::IFF_UP as u32, Some(Address::unsupported())),
            Row::up("eth0", "192.0.2.2"),
        ]);
        let result = list.read().expect("valid terminal row with null next");
        assert_eq!(result.len(), 1);
        assert_eq!(result.first().expect("interface").name, "eth0");
    }

    #[test]
    fn traversal_rejects_invalid_names_and_utf8() {
        for name in [vec![], vec![b'x'; libc::IFNAMSIZ], b"bad://name".to_vec(), vec![0xff]] {
            assert_invalid(&List::new(vec![Row::new(
                Some(&name),
                libc::IFF_UP as u32,
                Some(Address::ip("192.0.2.1")),
            )]));
        }
        let longest = "x".repeat(libc::IFNAMSIZ - 1);
        let result =
            List::new(vec![Row::up(&longest, "192.0.2.1")]).read().expect("longest supported name");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn traversal_filters_ipv4_ipv6_and_scoped_addresses() {
        let mut rows = [
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "127.0.0.1",
            "::",
            "ff02::1",
            "fe80::1",
            "::1",
        ]
        .into_iter()
        .map(|address| Row::up("ignored", address))
        .collect::<Vec<_>>();
        let mut scoped = Address::ip("2001:db8::1");
        if let Address::V6(address) = &mut scoped {
            address.sin6_scope_id = 7;
        }
        rows.push(Row::new(Some(b"scoped"), libc::IFF_UP as u32, Some(scoped)));
        rows.push(Row::up("eth0", "192.0.2.1"));
        rows.push(Row::up("eth0", "2001:db8::1"));
        let result = List::new(rows).read().expect("unscoped unicast rows");
        assert_eq!(result.len(), 1);
        let interface = result.first().expect("interface");
        assert_eq!(interface.name, "eth0");
        assert_eq!(
            interface.addresses,
            [
                "192.0.2.1".parse::<IpAddr>().expect("fixture"),
                "2001:db8::1".parse().expect("fixture")
            ]
        );
    }

    #[test]
    fn traversal_deduplicates_and_sorts_interfaces_and_addresses() {
        let rows = vec![
            Row::up("z0", "192.0.2.4"),
            Row::up("a0", "2001:db8::2"),
            Row::up("a0", "192.0.2.3"),
            Row::up("a0", "192.0.2.1"),
            Row::up("a0", "192.0.2.3"),
        ];
        let result = List::new(rows).read().expect("ordered interfaces");
        assert_eq!(
            result.iter().map(|value| value.name.as_str()).collect::<Vec<_>>(),
            ["a0", "z0"]
        );
        let addresses = &result.first().expect("first interface").addresses;
        let expected = ["192.0.2.1", "192.0.2.3", "2001:db8::2"]
            .map(|value| value.parse::<IpAddr>().expect("fixture"));
        assert_eq!(addresses, &expected);
    }

    #[test]
    fn row_limit_accepts_boundary_and_rejects_overflow_and_cycles() {
        let rows = (0..MAX_ROWS).map(|_| Row::up("eth0", "192.0.2.1")).collect();
        let result = List::new(rows).read().expect("exact row cap");
        assert_eq!(result.len(), 1);
        let rows = (0..=MAX_ROWS).map(|_| Row::up("eth0", "192.0.2.1")).collect();
        assert_invalid(&List::new(rows));
        let mut list = List::new(vec![Row::up("eth0", "192.0.2.1")]);
        list.cycle();
        assert_invalid(&list);
        // Skipped rows consume the same budget and cannot create an unbounded walk.
        let mut list = List::new(vec![Row::new(None, 0, None)]);
        list.cycle();
        assert_invalid(&list);
    }

    #[test]
    fn address_limit_counts_unique_addresses_and_rejects_overflow() {
        let rows = || {
            (1..=MAX_ADDRESSES)
                .map(|value| Row::up("eth0", &format!("192.0.2.{value}")))
                .collect::<Vec<_>>()
        };
        let mut exact = rows();
        exact.push(Row::up("eth0", "192.0.2.1"));
        let result = List::new(exact).read().expect("duplicate at exact address cap");
        assert_eq!(result.first().expect("interface").addresses.len(), MAX_ADDRESSES);
        let mut overflow = rows();
        overflow.push(Row::up("eth0", "198.51.100.1"));
        assert_invalid(&List::new(overflow));
    }

    #[test]
    fn interface_limit_accepts_boundary_and_rejects_overflow() {
        let rows = || {
            (0..MAX_INTERFACES)
                .map(|value| Row::up(&format!("eth{value}"), "192.0.2.1"))
                .collect::<Vec<_>>()
        };
        let result = List::new(rows()).read().expect("exact interface cap");
        assert_eq!(result.len(), MAX_INTERFACES);
        let mut overflow = rows();
        overflow.push(Row::up("extra", "192.0.2.1"));
        assert_invalid(&List::new(overflow));
    }

    #[test]
    fn only_unambiguous_non_loopback_unicast_addresses_are_selected() {
        for value in [
            "0.0.0.0",
            "255.255.255.255",
            "224.0.0.1",
            "::",
            "ff02::1",
            "fe80::1",
            "127.0.0.1",
            "::1",
        ] {
            assert!(!admissible(value.parse().expect("fixture")));
        }
        for value in ["192.0.2.1", "2001:db8::1"] {
            assert!(admissible(value.parse().expect("fixture")));
        }
    }
}
