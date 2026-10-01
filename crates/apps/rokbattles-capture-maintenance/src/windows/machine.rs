use super::{denied, peer, scm};
use crate::{
    lifecycle::Machine,
    rollback::{self, Content, Entry, Snapshot},
};
use rokbattles_capture_helper::windows::open_lock::OpenLock;
use rokbattles_capture_ipc::windows::{
    InstalledFile, ProtectedInstallation, installation_root, validate_protected_path,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::{
        ffi::OsStringExt,
        fs::OpenOptionsExt,
        io::{AsRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_SHARE_DELETE, GetFileInformationByHandle,
    },
    System::Threading::*,
};
const FILES: &[&str] = &[
    "rokbattles-capture-helper.exe",
    "rokbattles-desktop-agent.exe",
    "rokbattles-desktop.exe",
    r"capture\windivert\x86_64-pc-windows-msvc\WinDivert.dll",
    r"capture\windivert\x86_64-pc-windows-msvc\WinDivert64.sys",
];
pub struct NativeMachine {
    root: PathBuf,
    _root: OwnedHandle,
    parent: peer::Process,
    gate: Option<OpenLock>,
    retained: Vec<ProtectedInstallation>,
    exclusive: Vec<(String, Option<File>)>,
    retired: Vec<PathBuf>,
}
impl NativeMachine {
    pub fn new() -> io::Result<Self> {
        let root = installation_root()?;
        let root_handle = validate_protected_path(&root)?;
        Ok(Self {
            root,
            _root: root_handle,
            parent: peer::current_parent()?,
            gate: None,
            retained: Vec::new(),
            exclusive: Vec::new(),
            retired: Vec::new(),
        })
    }
    fn backup(&self) -> PathBuf {
        self.root.join(r".maintenance\rollback")
    }
    fn files(&self) -> Vec<&'static str> {
        FILES
            .iter()
            .copied()
            .filter(|name| cfg!(target_arch = "x86_64") || !name.starts_with("capture"))
            .collect()
    }
    fn lock_stopped_files(&mut self) -> io::Result<()> {
        let mut locked = Vec::new();
        for name in self.files() {
            let path = self.root.join(name);
            let file = if exists(&path)? {
                drop(validate_protected_path(&path)?);
                // Retain all originals before renaming ANY. Deny read/write sharing,
                // but allow our atomic retirement rename through FILE_SHARE_DELETE.
                Some(
                    OpenOptions::new()
                        .read(true)
                        .write(true)
                        .share_mode(FILE_SHARE_DELETE)
                        .open(path)
                        .map_err(|_error| {
                            io::Error::new(
                                io::ErrorKind::WouldBlock,
                                "capture image remains in use; restart required",
                            )
                        })?,
                )
            } else {
                None
            };
            locked.push((name.to_owned(), file));
        }
        self.exclusive = locked;
        Ok(())
    }
    fn retire_locked_files(&mut self) -> io::Result<()> {
        let directory=(0..32).map(|n|self.root.join(format!(r".maintenance\retired-{n}")))
            .find(|path| matches!(fs::symlink_metadata(path),Err(error) if error.kind()==io::ErrorKind::NotFound)).ok_or_else(denied)?;
        fs::create_dir(&directory)?;
        let _directory = validate_protected_path(&directory)?;
        for (index, (name, file)) in self.exclusive.iter().enumerate() {
            if file.is_some() {
                let target = directory.join(format!("{index}.bin"));
                fs::rename(self.root.join(name), &target)?;
                self.retired.push(target);
            }
        }
        Ok(())
    }
    fn verified_backup(&self) -> io::Result<Snapshot> {
        let directory = self.backup();
        let _directory = validate_protected_path(&directory)?;
        for entry in fs::read_dir(&directory)? {
            let _entry = validate_protected_path(&entry?.path())?;
        }
        Snapshot::read_verified(&directory, &self.files())
    }
    fn cleanup_backups(&mut self) -> io::Result<()> {
        self.exclusive.clear();
        let _snapshot = self.verified_backup()?;
        for entry in fs::read_dir(self.backup())? {
            fs::remove_file(entry?.path())?;
        }
        fs::remove_dir(self.backup())?;
        for n in 0..32 {
            let directory = self.root.join(format!(r".maintenance\retired-{n}"));
            if !exists(&directory)? {
                continue;
            }
            drop(validate_protected_path(&directory)?);
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                if !(0..self.files().len())
                    .any(|n| entry.file_name() == format!("{n}.bin").as_str())
                {
                    return Err(denied());
                }
                drop(validate_protected_path(&entry.path())?);
                fs::remove_file(entry.path())?;
            }
            fs::remove_dir(directory)?;
        }
        Ok(())
    }
    fn verify_new_apps(&mut self) -> io::Result<()> {
        let manifest = self.root.join(r".maintenance\payload.json");
        let _manifest = validate_protected_path(&manifest)?;
        let mut bytes = Vec::new();
        File::open(&manifest)?.take(8193).read_to_end(&mut bytes)?;
        if bytes.len() > 8192 {
            return Err(denied());
        }
        let data: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_error| denied())?;
        let object = data.as_object().ok_or_else(denied)?;
        if object.len() != 3 {
            return Err(denied());
        }
        for (name, kind) in [
            ("rokbattles-capture-helper.exe", InstalledFile::Helper),
            ("rokbattles-desktop-agent.exe", InstalledFile::Agent),
            ("rokbattles-capture-maintenance.next.exe", InstalledFile::StagedMaintenance),
        ] {
            let pin = object.get(name).and_then(|v| v.as_str()).ok_or_else(denied)?;
            if pin.len() != 64 || !pin.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(denied());
            }
            let file = ProtectedInstallation::open(kind)?;
            let mut digest = Sha256::new();
            let mut input = File::open(file.path())?;
            io::copy(&mut input, &mut DigestWriter(&mut digest))?;
            if format!("{:x}", digest.finalize()) != *pin {
                return Err(denied());
            }
            self.retained.push(file);
        }
        self.retained.push(ProtectedInstallation::open(InstalledFile::Desktop)?);
        Ok(())
    }
}
struct DigestWriter<'a>(&'a mut Sha256);
impl Write for DigestWriter<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}
impl Machine for NativeMachine {
    fn acquire_gate(&mut self) -> io::Result<()> {
        if self.gate.is_none() {
            self.gate = Some(OpenLock::acquire()?);
        }
        Ok(())
    }
    fn mark_unavailable(&mut self) -> io::Result<()> {
        let path = self.root.join(".capture-maintenance");
        if exists(&path)? {
            drop(validate_protected_path(&path)?);
            return Ok(());
        }
        let mut marker = OpenOptions::new().write(true).create_new(true).open(path)?;
        marker.write_all(b"ROKBattlesCaptureMaintenance:1\n")?;
        marker.sync_all()
    }
    fn validate_existing_services(&mut self) -> io::Result<()> {
        // Validate BOTH before stopping/changing either. Another app's WinDivert
        // service is a conflict, including a correctly named different version.
        scm::verify(scm::HELPER, &self.root)?;
        #[cfg(target_arch = "x86_64")]
        scm::verify(scm::DRIVER, &self.root)?;
        Ok(())
    }
    fn stop_helper(&mut self) -> io::Result<()> {
        self.retained.clear();
        scm::stop(scm::HELPER)
    }
    fn stop_driver(&mut self) -> io::Result<()> {
        #[cfg(target_arch = "x86_64")]
        scm::stop(scm::DRIVER)?;
        Ok(())
    }
    fn require_agents_exited(&mut self) -> io::Result<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let mut alive = false;
            for entry in peer::processes()? {
                let n = entry.szExeFile.iter().position(|x| *x == 0).ok_or_else(denied)?;
                let name =
                    std::ffi::OsString::from_wide(entry.szExeFile.get(..n).ok_or_else(denied)?);
                if name.to_string_lossy().eq_ignore_ascii_case("rokbattles-desktop-agent.exe") {
                    // SAFETY: query-only process; keep handle during image/liveness check.
                    let process = super::own(unsafe {
                        OpenProcess(
                            PROCESS_QUERY_LIMITED_INFORMATION | 0x0010_0000,
                            0,
                            entry.th32ProcessID,
                        )
                    })?;
                    if super::same_path(
                        &peer::image_path(process.as_raw_handle())?,
                        &self.root.join("rokbattles-desktop-agent.exe"),
                    ) {
                        // SAFETY: live synchronization process handle, zero wait.
                        alive |= unsafe {
                            WaitForSingleObject(process.as_raw_handle(), 0) == WAIT_TIMEOUT
                        };
                    }
                }
            }
            if !alive {
                match self.lock_stopped_files() {
                    Ok(()) => return Ok(()),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => return Err(error),
                }
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "close the ROK Battles background agent before maintenance",
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    fn backup_stopped_payload(&mut self) -> io::Result<()> {
        let backup = self.backup();
        if exists(&backup)? {
            let _verified = self.verified_backup()?;
        } else {
            fs::create_dir(&backup)?;
            let _directory = validate_protected_path(&backup)?;
            let mut snapshot = Snapshot { protocol: 1, entries: Vec::new() };
            for (index, (name, file)) in self.exclusive.iter_mut().enumerate() {
                let content = if let Some(file) = file {
                    let mut info = BY_HANDLE_FILE_INFORMATION::default();
                    // SAFETY: retained exclusive original and correctly sized output.
                    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
                        return Err(io::Error::last_os_error());
                    }
                    file.seek(SeekFrom::Start(0))?;
                    let (size, sha256) = rollback::hash(file)?;
                    file.seek(SeekFrom::Start(0))?;
                    let target = backup.join(format!("{index}.bin"));
                    let mut output =
                        OpenOptions::new().write(true).create_new(true).open(&target)?;
                    io::copy(file, &mut output)?;
                    output.sync_all()?;
                    Some(Content {
                        size,
                        sha256,
                        volume: info.dwVolumeSerialNumber,
                        file_index: (u64::from(info.nFileIndexHigh) << 32)
                            | u64::from(info.nFileIndexLow),
                    })
                } else {
                    None
                };
                snapshot.entries.push(Entry { name: name.clone(), content });
            }
            // Complete immutable copies + exact identities/presence are durable
            // BEFORE the first rename. A crash during retirement can always recover
            // these copies; an incomplete manifest is never accepted on retry.
            snapshot.write_complete(&backup)?;
            let _verified = self.verified_backup()?;
        }
        self.retire_locked_files()
    }
    fn validate_new_payload(&mut self) -> io::Result<()> {
        if !self.parent.alive() {
            return Err(denied());
        }
        self.verify_new_apps()?;
        #[cfg(target_arch = "x86_64")]
        {
            // Online trust is explicitly confined to the elevated installer.
            let driver = ProtectedInstallation::open(InstalledFile::WinDivertDriver)?;
            super::signature::verify_driver_online(driver.path())?;
            let trusted=rokbattles_capture_helper::windows::native_trust::TrustedWinDivert::verify_files_cached()?;
            drop(trusted);
            self.retained.push(ProtectedInstallation::open(InstalledFile::WinDivertDll)?);
            self.retained.push(driver);
        }
        Ok(())
    }
    fn register_services(&mut self) -> io::Result<()> {
        #[cfg(target_arch = "x86_64")]
        scm::register(scm::DRIVER, &self.root)?;
        scm::register(scm::HELPER, &self.root)
    }
    fn start_driver(&mut self) -> io::Result<()> {
        #[cfg(target_arch = "x86_64")]
        {
            scm::start(scm::DRIVER)?;
            scm::verify(scm::DRIVER, &self.root)?;
        }
        Ok(())
    }
    fn probe_passive_open(&mut self) -> io::Result<()> {
        #[cfg(target_arch = "x86_64")]
        {
            let trust =
                rokbattles_capture_helper::windows::native_trust::TrustedWinDivert::verify()?;
            // SAFETY: compiled exact upstream SHA/ABI, driver policy, fixed protected
            // path and retained no-write/delete handles checked immediately above.
            let library = unsafe {
                rokbattles_capture_adapters::windivert::WinDivert::load(trust.dll_path())
            }
            .map_err(|_error| denied())?;
            let capture = library.open().map_err(|_error| denied())?;
            // Passive probe reads no packets and closes immediately. It proves the
            // pre-provisioned driver accepts NO_INSTALL without implicit startup.
            drop(capture);
        }
        Ok(())
    }
    fn release_gate(&mut self) {
        self.gate.take();
    }
    fn start_helper(&mut self) -> io::Result<()> {
        scm::start(scm::HELPER)
    }
    fn mark_available(&mut self) -> io::Result<()> {
        if !self.parent.alive() {
            return Err(denied());
        }
        let ready = self.root.join(".capture-ready-v1");
        if exists(&ready)? {
            drop(validate_protected_path(&ready)?);
        }
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(ready)?;
        file.write_all(b"ROKBattlesCaptureReady:1\n")?;
        file.sync_all()?;
        drop(file);
        self.cleanup_backups()?;
        // The installer retains the durable block through maintenance self-swap
        // and atomic current.sha256 update. Only its fixed FINAL hook removes it.
        drop(validate_protected_path(&self.root.join(".capture-maintenance"))?);
        Ok(())
    }
    fn restore_stopped_payload(&mut self) -> io::Result<()> {
        self.retained.clear();
        let snapshot = self.verified_backup()?;
        // Acquire all current new images first; retire those too, so no open
        // handle can restart a partially rolled-back version from its old name.
        self.exclusive.clear();
        self.lock_stopped_files()?;
        self.retire_locked_files()?;
        for (index, entry) in snapshot.entries.iter().enumerate() {
            let target = self.root.join(&entry.name);
            if let Some(content) = &entry.content {
                let source = self.backup().join(format!("{index}.bin"));
                let _source = validate_protected_path(&source)?;
                let mut output = OpenOptions::new().write(true).create_new(true).open(&target)?;
                io::copy(&mut File::open(source)?, &mut output)?;
                output.sync_all()?;
                drop(output);
                let _target = validate_protected_path(&target)?;
                let (size, hash) = rollback::hash(&File::open(target)?)?;
                if size != content.size || hash != content.sha256 {
                    return Err(denied());
                }
            } else if exists(&target)? {
                return Err(denied());
            }
        }
        // Capture remains blocked, both services disabled, and the verified
        // recovery set survives. The UI/resources can require full repair.
        Ok(())
    }
    fn remove_services(&mut self) -> io::Result<()> {
        scm::remove(scm::HELPER)?;
        #[cfg(target_arch = "x86_64")]
        scm::remove(scm::DRIVER)?;
        Ok(())
    }
}
