//! Capability-relative, read-only mailcache access. Selected roots are opened
//! component by component without following links; file names cannot escape.

use std::{
    io::Read,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, bail};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, Metadata, OpenOptions, ReadDir};
use rokbattles_desktop_store::{FileSig, MAX_MAIL_BYTES, MAX_PATH_BYTES};
use zeroize::Zeroizing;

pub struct MailRoot {
    path: PathBuf,
    directory: Dir,
}

pub struct Scan {
    entries: ReadDir,
    visited: usize,
}

pub struct Observation {
    pub path: PathBuf,
    pub sig: FileSig,
}

impl MailRoot {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let text = path.to_str().context("mailcache root must be UTF-8")?;
        if !path.is_absolute() || text.len() > MAX_PATH_BYTES || text.contains('\0') {
            bail!("invalid mailcache root");
        }
        let mut components = path.components();
        #[cfg(windows)]
        let anchor = {
            use std::path::Prefix;
            let Some(Component::Prefix(prefix)) = components.next() else {
                bail!("mailcache root must be a local drive path");
            };
            if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
                bail!("network and device mailcache roots are unsupported");
            }
            PathBuf::from(prefix.as_os_str()).join("\\")
        };
        #[cfg(not(windows))]
        let anchor = PathBuf::from("/");
        if !matches!(components.next(), Some(Component::RootDir)) {
            bail!("mailcache root must be absolute");
        }
        let mut directory = Dir::open_ambient_dir(anchor, cap_std::ambient_authority())?;
        for component in components {
            let Component::Normal(name) = component else {
                bail!("mailcache root contains an alias component");
            };
            // Retaining each directory handle prevents an ancestor rename/link
            // race from redirecting the subsequent relative open elsewhere.
            directory = directory.open_dir_nofollow(name)?;
        }
        Ok(Self { path: path.to_path_buf(), directory })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn scan(&self) -> anyhow::Result<Scan> {
        Ok(Scan { entries: self.directory.entries()?, visited: 0 })
    }

    pub fn scan_next(
        &self,
        scan: &mut Scan,
        now_ms: u64,
    ) -> anyhow::Result<(Vec<Observation>, bool)> {
        let mut output = Vec::new();
        for _ in 0..256 {
            let Some(entry) = scan.entries.next() else {
                return Ok((output, true));
            };
            scan.visited += 1;
            if scan.visited > 200_000 {
                bail!("mailcache scan entry limit reached");
            }
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str().filter(|name| mail_id(name).is_some()) else {
                continue;
            };
            let metadata = self.directory.symlink_metadata(name)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                continue;
            }
            let Some(sig) = signature(&metadata) else {
                continue;
            };
            if now_ms.saturating_sub(sig.modified_ms) < 1500 {
                continue;
            }
            let path = self.path.join(name);
            if path.as_os_str().len() <= MAX_PATH_BYTES {
                output.push(Observation { path, sig });
            }
        }
        Ok((output, false))
    }

    pub fn read(&self, path: &Path, expected: FileSig) -> anyhow::Result<Zeroizing<Vec<u8>>> {
        if expected.size > MAX_MAIL_BYTES || expected.modified_ms > i64::MAX as u64 {
            bail!("invalid expected mailcache signature");
        }
        if path.parent() != Some(self.path()) {
            bail!("mailcache file is outside selected root");
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| mail_id(name).is_some())
            .context("invalid mailcache name")?;
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let file = self.directory.open_with(name, &options)?;
        let before = file.metadata()?;
        if signature(&before) != Some(expected) || !before.is_file() {
            bail!("mailcache file changed");
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(usize::try_from(expected.size)?));
        (&file).take(MAX_MAIL_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != expected.size || signature(&file.metadata()?) != Some(expected) {
            bail!("mailcache file changed while reading");
        }
        Ok(bytes)
    }
}

fn signature(metadata: &Metadata) -> Option<FileSig> {
    let size = metadata.len();
    if size > MAX_MAIL_BYTES {
        return None;
    }
    let modified_ms = u64::try_from(
        metadata
            .modified()
            .ok()?
            .into_std()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis(),
    )
    .ok()?;
    Some(FileSig { size, modified_ms })
}

pub fn mail_id(name: &str) -> Option<u128> {
    let value = name.strip_prefix("Persistent.Mail.")?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// UI deduplication hint only, never an authorization or root-routing key.
pub fn identity(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) { text.to_lowercase() } else { text.into_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_numeric_mail_names_are_eligible() {
        assert_eq!(mail_id("Persistent.Mail.123"), Some(123));
        for name in [
            "Persistent.Mail.",
            "Persistent.Mail.1:stream",
            "Persistent.Mail.1/child",
            "other",
            "Persistent.Mail.99999999999999999999999999999999999999999999",
        ] {
            assert!(mail_id(name).is_none());
        }
    }

    #[test]
    fn scans_incrementally_and_revalidates_before_read() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().canonicalize().expect("canonical");
        let file = path.join("Persistent.Mail.123");
        std::fs::write(&file, b"fixture").expect("fixture");
        std::fs::write(path.join("unrelated"), b"never uploaded").expect("fixture");
        let root = MailRoot::open(&path).expect("root");
        let (items, done) =
            root.scan_next(&mut root.scan().expect("scan"), u64::MAX).expect("next");
        assert!(done);
        assert_eq!(items.len(), 1);
        let item = items.first().expect("mail");
        assert_eq!(&**root.read(&item.path, item.sig).expect("read"), b"fixture");
        std::fs::write(&file, b"changed fixture").expect("change");
        root.read(&item.path, item.sig).expect_err("changed file");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_roots_files_and_fifo_are_not_read() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().canonicalize().expect("canonical");
        std::fs::create_dir(path.join("root")).expect("root");
        symlink(path.join("root"), path.join("alias")).expect("link");
        assert!(MailRoot::open(&path.join("alias")).is_err());
        std::fs::write(path.join("secret"), b"secret").expect("fixture");
        symlink(path.join("secret"), path.join("root/Persistent.Mail.1")).expect("link");
        let root = MailRoot::open(&path.join("root")).expect("root");
        assert!(
            root.scan_next(&mut root.scan().expect("scan"), u64::MAX).expect("next").0.is_empty()
        );
        root.read(&path.join("root/Persistent.Mail.1"), FileSig { size: 6, modified_ms: 0 })
            .expect_err("symlink file");
    }
}
