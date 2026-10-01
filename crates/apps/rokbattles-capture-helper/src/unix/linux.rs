//! Host /proc socket attribution. No external utilities, caller PID, namespace
//! switching, executable paths, or cached authorization are used here.
use crate::ownership::{OwnerLookup, OwnershipError, TerminalOwnership};
use rokbattles_capture_runtime::packet::FlowKey;
use std::{
    fs,
    io::{self, Read},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

const MAX_PROCESSES: usize = 4096;
const MAX_FDS: usize = 4096;
const MAX_TABLE_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    uid: u32,
    start: u64,
    namespace: (u64, u64),
}
#[derive(Clone, PartialEq, Eq)]
pub struct SocketOwner {
    process: ProcessIdentity,
    fd: u32,
    inode: u64,
}
pub struct UnixOwnerLookup {
    uid: u32,
}
impl UnixOwnerLookup {
    pub fn new(uid: u32) -> Self {
        Self { uid }
    }
}
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "socket ownership unavailable")
}
fn read_bounded(path: &Path, limit: u64) -> io::Result<String> {
    let mut text = String::new();
    fs::File::open(path)?.take(limit + 1).read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(denied());
    }
    Ok(text)
}
fn namespace(path: &Path) -> io::Result<(u64, u64)> {
    let metadata = fs::metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}
fn process_path(pid: u32) -> PathBuf {
    Path::new("/proc").join(pid.to_string())
}
impl ProcessIdentity {
    pub fn read(pid: u32, uid: u32) -> io::Result<Self> {
        if pid == 0 || uid == 0 {
            return Err(denied());
        }
        let root = process_path(pid);
        let host = namespace(Path::new("/proc/self/ns/net"))?;
        let actual = namespace(&root.join("ns/net"))?;
        if actual != host {
            return Err(denied());
        }
        let first = parse_stat(&read_bounded(&root.join("stat"), 8192)?, pid)?;
        if !same_uid(&read_bounded(&root.join("status"), 65536)?, uid) {
            return Err(denied());
        }
        let second = parse_stat(&read_bounded(&root.join("stat"), 8192)?, pid)?;
        if first != second || namespace(&root.join("ns/net"))? != host {
            return Err(denied());
        }
        Ok(Self { pid, uid, start: first, namespace: actual })
    }
    pub fn is_alive(&self) -> bool {
        Self::read(self.pid, self.uid).is_ok_and(|current| current == *self)
    }
}
fn same_uid(status: &str, uid: u32) -> bool {
    let mut lines = status.lines().filter_map(|line| line.strip_prefix("Uid:"));
    let Some(line) = lines.next() else {
        return false;
    };
    let values =
        line.split_ascii_whitespace().map(str::parse::<u32>).collect::<Result<Vec<_>, _>>();
    lines.next().is_none()
        && values.is_ok_and(|values| values.len() == 4 && values.iter().all(|value| *value == uid))
}
fn parse_stat(stat: &str, pid: u32) -> io::Result<u64> {
    let (prefix, suffix) = stat.rsplit_once(") ").ok_or_else(denied)?;
    if prefix.split_once(" (").and_then(|(number, _)| number.parse::<u32>().ok()) != Some(pid) {
        return Err(denied());
    }
    let mut fields = suffix.split_ascii_whitespace();
    if matches!(fields.next(), None | Some("Z" | "X" | "x")) {
        return Err(denied());
    }
    // State is field 3. Starttime is field 22; comm may itself contain spaces/').'
    fields.nth(18).ok_or_else(denied)?.parse::<u64>().map_err(|_error| denied())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    key: FlowKey,
    uid: u32,
    inode: u64,
    state: u8,
}
fn endpoint(text: &str) -> io::Result<SocketAddr> {
    let (address, port) = text.split_once(':').ok_or_else(denied)?;
    let port = u16::from_str_radix(port, 16).map_err(|_error| denied())?;
    let ip = match address.len() {
        8 => IpAddr::V4(Ipv4Addr::from(
            u32::from_str_radix(address, 16).map_err(|_error| denied())?.to_ne_bytes(),
        )),
        32 => {
            let mut bytes = [0; 16];
            for (index, target) in bytes.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let word = address.get(index * 8..index * 8 + 8).ok_or_else(denied)?;
                target.copy_from_slice(
                    &u32::from_str_radix(word, 16).map_err(|_error| denied())?.to_ne_bytes(),
                );
            }
            Ipv6Addr::from(bytes)
                .to_ipv4_mapped()
                .map_or_else(|| IpAddr::V6(Ipv6Addr::from(bytes)), IpAddr::V4)
        }
        _ => return Err(denied()),
    };
    Ok(SocketAddr::new(ip, port))
}
fn row(text: &str) -> io::Result<Row> {
    let columns: Vec<_> = text.split_ascii_whitespace().take(11).collect();
    let field = |index| columns.get(index).copied().ok_or_else(denied);
    Ok(Row {
        key: FlowKey { client: endpoint(field(1)?)?, server: endpoint(field(2)?)? },
        state: u8::from_str_radix(field(3)?, 16).map_err(|_error| denied())?,
        uid: field(7)?.parse().map_err(|_error| denied())?,
        inode: field(9)?.parse().map_err(|_error| denied())?,
    })
}
fn exact_row(table: &str, key: FlowKey) -> io::Result<Option<Row>> {
    let mut lines = table.lines();
    if !lines
        .next()
        .is_some_and(|header| header.contains("local_address") && header.contains("inode"))
    {
        return Err(denied());
    }
    let mut matched = None;
    for line in lines {
        let candidate = row(line)?;
        if candidate.key != key {
            continue;
        }
        if matched.replace(candidate).is_some() {
            return Err(denied());
        }
    }
    Ok(matched)
}
fn current_row(key: FlowKey) -> io::Result<Option<Row>> {
    // IPv4-mapped IPv6 sockets emit IPv4 packets. Both kernel tables must be
    // checked, including before claiming a retired tuple is absent.
    let four = exact_row(&read_bounded(Path::new("/proc/self/net/tcp"), MAX_TABLE_BYTES)?, key)?;
    let six = exact_row(&read_bounded(Path::new("/proc/self/net/tcp6"), MAX_TABLE_BYTES)?, key)?;
    match (four, six) {
        (Some(_), Some(_)) => Err(denied()),
        (four, six) => Ok(four.or(six)),
    }
}

fn fd_inode(pid: u32, fd: u32) -> io::Result<u64> {
    let target = fs::read_link(process_path(pid).join("fd").join(fd.to_string()))?;
    let text = target.to_str().ok_or_else(denied)?;
    text.strip_prefix("socket:[")
        .and_then(|text| text.strip_suffix(']'))
        .ok_or_else(denied)?
        .parse()
        .map_err(|_error| denied())
}
fn socket_state_live(state: u8) -> bool {
    matches!(state, 1..=5 | 8..=9 | 11)
}
impl OwnerLookup for UnixOwnerLookup {
    type Owner = SocketOwner;
    fn owner_for_syn(&mut self, key: FlowKey) -> Result<Option<SocketOwner>, OwnershipError> {
        self.find_owner(key).map_err(|_error| OwnershipError)
    }
    fn terminal_owner(
        &mut self,
        key: FlowKey,
        owner: &SocketOwner,
    ) -> Result<TerminalOwnership, OwnershipError> {
        let row = current_row(key).map_err(|_error| OwnershipError)?;
        let result = terminal_row(row, self.uid, owner.inode);
        if result == TerminalOwnership::Owned && !self.still_owner(key, owner) {
            return Ok(TerminalOwnership::Conflict);
        }
        Ok(result)
    }
    fn alive(&self, owner: &SocketOwner) -> bool {
        owner.process.is_alive()
    }
    fn still_owner(&mut self, key: FlowKey, owner: &SocketOwner) -> bool {
        if !owner.process.is_alive()
            || fd_inode(owner.process.pid, owner.fd).ok() != Some(owner.inode)
        {
            return false;
        }
        let valid = current_row(key).is_ok_and(|row| {
            row.is_some_and(|row| {
                row.uid == self.uid && row.inode == owner.inode && socket_state_live(row.state)
            })
        });
        valid
            && owner.process.is_alive()
            && fd_inode(owner.process.pid, owner.fd).ok() == Some(owner.inode)
    }
}
fn terminal_row(row: Option<Row>, uid: u32, inode: u64) -> TerminalOwnership {
    match row {
        None => TerminalOwnership::Absent,
        Some(row) if row.uid != uid || row.inode != inode => TerminalOwnership::Conflict,
        Some(row) if matches!(row.state, 6 | 7 | 10) => TerminalOwnership::Absent,
        Some(row) if socket_state_live(row.state) => TerminalOwnership::Owned,
        _ => TerminalOwnership::Conflict,
    }
}

impl UnixOwnerLookup {
    fn find_owner(&self, key: FlowKey) -> io::Result<Option<SocketOwner>> {
        let Some(row) = current_row(key)? else {
            return Ok(None);
        };
        if row.uid != self.uid {
            return Ok(None);
        }
        if row.inode == 0 || !matches!(row.state, 1 | 2) {
            return Err(denied());
        }
        let mut matched = None;
        let mut process_count = 0;
        let mut budget = super::WorkBudget::new();
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok())
            else {
                continue;
            };
            budget.consume(1)?;
            process_count += 1;
            if process_count > MAX_PROCESSES {
                return Err(denied());
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            if metadata.uid() != self.uid {
                continue;
            }
            let process = ProcessIdentity::read(pid, self.uid)?;
            for (count, fd) in fs::read_dir(entry.path().join("fd"))?.enumerate() {
                budget.consume(1)?;
                if count >= MAX_FDS {
                    return Err(denied());
                }
                let fd = fd?;
                let fd = fd
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                    .ok_or_else(denied)?;
                let target = fs::read_link(entry.path().join("fd").join(fd.to_string()))?;
                if target.to_str() != Some(format!("socket:[{}]", row.inode).as_str()) {
                    continue;
                }
                // A duplicate FD or forked holder is ambiguous, even within one UID.
                if matched.replace(SocketOwner { process, fd, inode: row.inode }).is_some() {
                    return Err(denied());
                }
            }
        }
        let Some(owner) = matched else {
            return Err(denied());
        };
        if !owner.process.is_alive()
            || fd_inode(owner.process.pid, owner.fd)? != row.inode
            || current_row(key)?.is_none_or(|current| {
                current.uid != row.uid
                    || current.inode != row.inode
                    || !socket_state_live(current.state)
            })
        {
            return Err(denied());
        }
        Ok(Some(owner))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn table(rows: &str) -> String {
        format!(
            " sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n{rows}"
        )
    }
    fn key() -> FlowKey {
        FlowKey {
            client: "192.0.2.2:45000".parse().expect("client"),
            server: "198.51.100.1:3101".parse().expect("server"),
        }
    }
    const ROW: &str =
        " 0: 020200C0:AFC8 016433C6:0C1D 02 00000000:00000000 00:00000000 00000000 1000 0 345 1\n";
    #[test]
    fn kernel_table_endianness_uid_inode_and_duplicates_are_exact() {
        let row = exact_row(&table(ROW), key()).expect("valid").expect("match");
        assert_eq!((row.uid, row.inode, row.state), (1000, 345, 2));
        exact_row(&table(&ROW.repeat(2)), key()).expect_err("duplicate row");
        exact_row(&table("partial"), key()).expect_err("truncated row");
        exact_row("", key()).expect_err("missing table");
        assert_eq!(
            endpoint("0000000000000000FFFF0000020200C0:AFC8").expect("mapped").to_string(),
            "192.0.2.2:45000"
        );
        assert_eq!(
            endpoint("00000000000000000000000001000000:0C1D").expect("v6").to_string(),
            "[::1]:3101"
        );
    }
    #[test]
    fn removed_reset_row_is_distinct_from_reassigned_or_unavailable_ownership() {
        assert_eq!(terminal_row(None, 1000, 345), TerminalOwnership::Absent);
        let mut present = row(ROW).expect("fixture");
        assert_eq!(terminal_row(Some(present), 1000, 345), TerminalOwnership::Owned);
        present.state = 7;
        assert_eq!(terminal_row(Some(present), 1000, 345), TerminalOwnership::Absent);
        present.uid = 1001;
        assert_eq!(terminal_row(Some(present), 1000, 345), TerminalOwnership::Conflict);
        present.uid = 1000;
        present.inode = 346;
        assert_eq!(terminal_row(Some(present), 1000, 345), TerminalOwnership::Conflict);
        // A failed/truncated table never becomes None or a terminal observation.
        exact_row("partial kernel result", key()).expect_err("unavailable evidence");
    }

    #[test]
    fn process_starttime_and_every_uid_are_required() {
        let stat = format!("123 (name with ) spaces) S {} 9876 0", vec!["0"; 18].join(" "));
        assert_eq!(parse_stat(&stat, 123).expect("start"), 9876);
        parse_stat(&stat, 124).expect_err("wrong PID");
        parse_stat(&stat.replace(") S ", ") Z "), 123).expect_err("zombie");
        assert!(same_uid("Name: game\nUid:\t1000\t1000\t1000\t1000\n", 1000));
        assert!(!same_uid("Uid:\t1000\t1001\t1000\t1000\n", 1000));
        assert!(!same_uid("Uid: 1000 1000 1000 1000\nUid: 1000 1000 1000 1000", 1000));
        assert!(!socket_state_live(6)); // TIME_WAIT has no live owning FD.
        assert!(socket_state_live(4)); // FIN_WAIT_1 preserves the server tail.
        assert!(socket_state_live(5)); // FIN_WAIT_2 preserves the server tail.
    }
}
