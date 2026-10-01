//! Exact, bounded rollback manifests; no path from the manifest becomes a path
//! unless it equals the caller's compiled file list. Native code owns ACL checks.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub content: Option<Content>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub size: u64,
    pub sha256: String,
    pub volume: u32,
    pub file_index: u64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub protocol: u32,
    pub entries: Vec<Entry>,
}

pub fn hash(mut file: &File) -> io::Result<(u64, String)> {
    let mut digest = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size = size.checked_add(count as u64).ok_or_else(invalid)?;
        if size > 512 * 1024 * 1024 {
            return Err(invalid());
        }
        digest.update(buffer.get(..count).ok_or_else(invalid)?);
    }
    Ok((size, format!("{:x}", digest.finalize())))
}
impl Snapshot {
    pub fn write_complete(&self, dir: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(|_error| invalid())?;
        let pending = dir.join("manifest.pending");
        let mut file = OpenOptions::new().write(true).create_new(true).open(&pending)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(pending, dir.join("manifest.json"))
    }
    pub fn read_verified(dir: &Path, expected: &[&str]) -> io::Result<Self> {
        let mut bytes = Vec::new();
        File::open(dir.join("manifest.json"))?.take(16385).read_to_end(&mut bytes)?;
        if bytes.len() > 16384 {
            return Err(invalid());
        }
        let snapshot: Self = serde_json::from_slice(&bytes).map_err(|_error| invalid())?;
        if snapshot.protocol != 1 || snapshot.entries.len() != expected.len() {
            return Err(invalid());
        }
        for (index, (entry, name)) in snapshot.entries.iter().zip(expected).enumerate() {
            if entry.name != *name {
                return Err(invalid());
            }
            let path = dir.join(format!("{index}.bin"));
            match &entry.content {
                Some(content) => {
                    if content.sha256.len() != 64
                        || !content.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(invalid());
                    }
                    let (size, hash) = hash(&File::open(path)?)?;
                    if size != content.size || hash != content.sha256 {
                        return Err(invalid());
                    }
                }
                None => {
                    if fs::symlink_metadata(path).is_ok() {
                        return Err(invalid());
                    }
                }
            }
        }
        // Reject unexpected, stale or partial files rather than silently adopting
        // an arbitrary directory from a previous/crashed installer generation.
        let mut allowed = vec!["manifest.json".to_owned()];
        for (i, entry) in snapshot.entries.iter().enumerate() {
            if entry.content.is_some() {
                allowed.push(format!("{i}.bin"));
            }
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || !allowed.iter().any(|name| entry.file_name() == name.as_str())
            {
                return Err(invalid());
            }
        }
        Ok(snapshot)
    }
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "rollback snapshot is incomplete or changed")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(dir: &Path) -> Snapshot {
        fs::write(dir.join("0.bin"), b"previous executable").expect("fixture");
        let (size, sha256) = hash(&File::open(dir.join("0.bin")).expect("fixture")).expect("hash");
        Snapshot {
            protocol: 1,
            entries: vec![
                Entry {
                    name: "helper".into(),
                    content: Some(Content { size, sha256, volume: 1, file_index: 2 }),
                },
                Entry { name: "driver".into(), content: None },
            ],
        }
    }
    #[test]
    fn partial_backup_is_never_a_rollback_set() {
        let dir = tempfile::tempdir().expect("fixture");
        let _snapshot = snapshot(dir.path());
        Snapshot::read_verified(dir.path(), &["helper", "driver"])
            .expect_err("invalid rollback rejected");
    }
    #[test]
    fn complete_backup_records_absence_and_rejects_tamper() {
        let dir = tempfile::tempdir().expect("fixture");
        snapshot(dir.path()).write_complete(dir.path()).expect("complete");
        let read = Snapshot::read_verified(dir.path(), &["helper", "driver"]).expect("valid");
        assert!(read.entries.get(1).expect("driver").content.is_none());
        fs::write(dir.path().join("0.bin"), b"changed").expect("tamper");
        Snapshot::read_verified(dir.path(), &["helper", "driver"])
            .expect_err("invalid rollback rejected");
    }
    #[test]
    fn stale_wrong_layout_and_extra_files_fail_closed() {
        let dir = tempfile::tempdir().expect("fixture");
        snapshot(dir.path()).write_complete(dir.path()).expect("complete");
        Snapshot::read_verified(dir.path(), &["new-helper", "driver"])
            .expect_err("invalid rollback rejected");
        fs::write(dir.path().join("1.bin"), b"unexpected old driver").expect("fixture");
        Snapshot::read_verified(dir.path(), &["helper", "driver"])
            .expect_err("invalid rollback rejected");
    }
}
