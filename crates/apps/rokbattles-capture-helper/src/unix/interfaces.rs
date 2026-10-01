//! Enumerate local interfaces through the OS; callers cannot supply names/addresses.
use std::{
    collections::BTreeMap,
    ffi::CStr,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
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
    struct Allocation(*mut libc::ifaddrs);
    impl Drop for Allocation {
        fn drop(&mut self) {
            // SAFETY: exact getifaddrs allocation, freed once after iteration ends.
            unsafe { libc::freeifaddrs(self.0) };
        }
    }
    let _allocation = Allocation(head);
    let mut interfaces = BTreeMap::<String, Vec<IpAddr>>::new();
    let mut cursor = head;
    let mut count = 0;
    while !cursor.is_null() {
        count += 1;
        if count > MAX_ROWS {
            return Err(invalid());
        }
        // SAFETY: a node in the live OS-owned getifaddrs list.
        let row = unsafe { &*cursor };
        cursor = row.ifa_next;
        if row.ifa_flags & libc::IFF_UP as u32 == 0
            || row.ifa_flags & libc::IFF_LOOPBACK as u32 != 0
            || row.ifa_addr.is_null()
            || row.ifa_name.is_null()
        {
            continue;
        }
        // SAFETY: getifaddrs supplies a NUL-terminated name for each row.
        let name = unsafe { CStr::from_ptr(row.ifa_name) }.to_str().map_err(|_error| invalid())?;
        if name.is_empty() || name.len() >= libc::IFNAMSIZ || name.contains("://") {
            return Err(invalid());
        }
        // SAFETY: non-null sockaddr is part of this live getifaddrs allocation.
        let family = unsafe { (*row.ifa_addr).sa_family } as i32;
        let address = match family {
            libc::AF_INET => {
                // SAFETY: AF_INET identifies the native sockaddr_in layout.
                let address = unsafe { &*row.ifa_addr.cast::<libc::sockaddr_in>() };
                IpAddr::V4(Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes()))
            }
            libc::AF_INET6 => {
                // SAFETY: AF_INET6 identifies the native sockaddr_in6 layout.
                let address = unsafe { &*row.ifa_addr.cast::<libc::sockaddr_in6>() };
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
