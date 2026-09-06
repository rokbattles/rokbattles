//! Privileged startup followed by irreversible privilege removal.

use std::{
    fs,
    io::{self, Write},
    net::{SocketAddr, SocketAddrV4, ToSocketAddrs},
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, bail};

use crate::{config::Config, rules};

/// Resolve the configured host once per startup. Existing conntrack entries
/// keep their old target when a later restart chooses a different address.
pub fn resolve(config: &Config) -> anyhow::Result<SocketAddrV4> {
    config
        .upstream_addr
        .to_socket_addrs()?
        .find_map(|address| match address {
            SocketAddr::V4(address)
                if public_target(*address.ip()) && address.ip() != config.bind_addr.ip() =>
            {
                Some(address)
            }
            _ => None,
        })
        .context("UPSTREAM_ADDR did not resolve to a usable IPv4 target")
}
fn nft(arguments: &[&str], input: Option<&str>) -> anyhow::Result<std::process::Output> {
    let path = ["/usr/sbin/nft", "/sbin/nft", "/usr/bin/nft"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .context("install nftables on the gateway node")?;
    let mut command = Command::new(path);
    command
        .env_clear()
        .args(arguments)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    if let Some(input) = input {
        child.stdin.take().context("nft stdin")?.write_all(input.as_bytes())?;
    }
    Ok(child.wait_with_output()?)
}

/// Validate table ownership, then commit only this application's rules atomically.
/// Does not flush conntrack or register shutdown cleanup.
pub fn install(config: &Config, upstream: SocketAddrV4) -> anyhow::Result<()> {
    if fs::read_to_string("/proc/sys/net/ipv6/conf/all/forwarding")
        .is_ok_and(|value| value.trim() != "0")
    {
        bail!("this IPv4 gateway requires IPv6 forwarding disabled on its dedicated node");
    }
    let listing = nft(&["-j", "list", "tables"], None)?;
    if !listing.status.success() {
        bail!("cannot inspect nftables: {}", String::from_utf8_lossy(&listing.stderr));
    }
    let tables: serde_json::Value = serde_json::from_slice(&listing.stdout)?;
    let exists = tables
        .get("nftables")
        .and_then(serde_json::Value::as_array)
        .context("invalid nft table listing")?
        .iter()
        .any(|entry| {
            entry.pointer("/table/family").and_then(serde_json::Value::as_str) == Some("ip")
                && entry.pointer("/table/name").and_then(serde_json::Value::as_str)
                    == Some(rules::TABLE_NAME)
        });
    let program = if exists {
        let listing =
            nft(&["-j", "list", "chain", "ip", rules::TABLE_NAME, "schema_identity"], None)?;
        let value: serde_json::Value = serde_json::from_slice(&listing.stdout)
            .context("owned table has no schema identity; refusing replacement")?;
        let valid = listing.status.success()
            && value.get("nftables").and_then(serde_json::Value::as_array).is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry.pointer("/chain/comment").and_then(serde_json::Value::as_str)
                        == Some(rules::SCHEMA_IDENTITY)
                })
            });
        if !valid {
            bail!("incompatible gateway table; refusing replacement");
        }
        rules::render_replace(config.bind_addr, upstream)
    } else {
        rules::render_create(config.bind_addr, upstream)
    };
    let result = nft(&["--check", "-f", "-"], Some(&program))?;
    if !result.status.success() {
        bail!("nftables validation failed: {}", String::from_utf8_lossy(&result.stderr));
    }
    // Install the fail-closed forwarding policy before enabling routing.
    let result = nft(&["-f", "-"], Some(&program))?;
    if !result.status.success() {
        bail!("nftables installation failed: {}", String::from_utf8_lossy(&result.stderr));
    }
    fs::write("/proc/sys/net/ipv4/ip_forward", b"1\n").context("cannot enable IPv4 forwarding")?;
    Ok(())
}

/// Drop root and all Linux capabilities before any network payload is parsed.
/// The service unit supplies an isolated account through GATEWAY_UID/GID.
pub fn drop_privileges(uid: u32, gid: u32) -> io::Result<()> {
    if uid == 0 || gid == 0 {
        return Err(io::Error::other("gateway worker UID/GID must be nonzero"));
    }
    fn check(value: i32) -> io::Result<()> {
        if value == -1 { Err(io::Error::last_os_error()) } else { Ok(()) }
    }
    // SAFETY: prctl arguments enable a documented one-way privilege restriction.
    check(unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) })?;
    // SAFETY: a zero length allows a null groups pointer and clears all groups.
    check(unsafe { libc::setgroups(0, std::ptr::null()) })?;
    // SAFETY: setresgid accepts numeric IDs, with no pointer arguments.
    check(unsafe { libc::setresgid(gid, gid, gid) })?;
    // SAFETY: setresuid drops all saved root IDs and their effective capabilities.
    check(unsafe { libc::setresuid(uid, uid, uid) })?;
    // An ambient startup capability also enters the inheritable set. Changing
    // UID clears effective, permitted and ambient capabilities, but leaves that
    // inheritable set behind. Clear all sets before reading network payloads.
    #[repr(C)]
    struct CapabilityHeader {
        version: u32,
        pid: i32,
    }
    #[repr(C)]
    struct CapabilityData {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }
    let header = CapabilityHeader { version: 0x2008_0522, pid: 0 };
    let data = [const { CapabilityData { effective: 0, permitted: 0, inheritable: 0 } }; 2];
    // SAFETY: Linux capability ABI v3 takes a header and two capability words.
    // Both pointers remain valid for the syscall, which only changes this thread.
    let result = unsafe { libc::syscall(libc::SYS_capset, &header, data.as_ptr()) };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: prctl disables process dumps and same-UID ptrace access.
    check(unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) })?;
    let status = fs::read_to_string("/proc/self/status")?;
    for key in ["CapEff:", "CapPrm:", "CapInh:", "CapAmb:"] {
        if !status
            .lines()
            .find(|line| line.starts_with(key))
            .is_some_and(|line| line.split_whitespace().nth(1) == Some("0000000000000000"))
        {
            return Err(io::Error::other("capabilities remained after dropping root"));
        }
    }
    Ok(())
}

/// Look up the dedicated system account installed by systemd-sysusers.
/// Explicit IDs support container deployments with their own account database.
pub fn worker_identity() -> anyhow::Result<(u32, u32)> {
    match (std::env::var("GATEWAY_UID"), std::env::var("GATEWAY_GID")) {
        (Ok(uid), Ok(gid)) => return Ok((uid.parse()?, gid.parse()?)),
        (Err(_), Err(_)) => {}
        _ => bail!("set both GATEWAY_UID and GATEWAY_GID, or neither"),
    }
    // SAFETY: the NUL-terminated constant is valid; startup is single-threaded,
    // and both numeric fields are copied before another account lookup.
    let entry = unsafe { libc::getpwnam(c"rokb-gateway".as_ptr()) };
    if entry.is_null() {
        bail!("install the rokb-gateway system account first");
    }
    // SAFETY: getpwnam returned a non-null passwd pointer valid until next lookup.
    let uid = unsafe { (*entry).pw_uid };
    // SAFETY: no account lookup has invalidated the pointer.
    let gid = unsafe { (*entry).pw_gid };
    Ok((uid, gid))
}

fn public_target(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, c, _d] = ip.octets();
    !matches!(a, 0 | 10 | 127 | 224..=255)
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 169 && b == 254)
        && !(a == 172 && (16..=31).contains(&b))
        && !(a == 192 && (b == 168 || (b == 0 && matches!(c, 0 | 2))))
        && !(a == 198 && (matches!(b, 18 | 19) || (b == 51 && c == 100)))
        && !(a == 203 && b == 0 && c == 113)
}

/// Reject a spool path that another local account could replace or modify.
pub fn validate_spool(path: &Path, uid: u32) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata =
        fs::symlink_metadata(path).context("install the gateway state directory first")?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        bail!(
            "gateway state directory must be a real directory owned by its worker with mode 0700"
        );
    }
    Ok(())
}

/// Prevent core dumps before configuration brings the ingress token into memory.
pub fn disable_core_dumps() -> io::Result<()> {
    let limit = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: limit is initialized and points to a valid rlimit structure.
    if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &raw const limit) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dns_answers_cannot_redirect_the_gateway_to_internal_or_reserved_networks() {
        for ip in [
            "0.1.2.3",
            "10.0.0.1",
            "100.64.1.1",
            "127.0.0.1",
            "169.254.169.254",
            "172.16.1.1",
            "192.168.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(!public_target(ip.parse().expect("fixture IP")), "{ip}");
        }
        assert!(public_target("8.8.8.8".parse().expect("public IP")));
    }
}
