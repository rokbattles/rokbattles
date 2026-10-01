//! Protected fixed install layout. No environment variables or caller paths are trusted.
use super::{LocalAllocation, denied, own, sid_string};
use std::{
    ffi::c_void,
    io,
    mem::{offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, BorrowedHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{ERROR_SUCCESS, HANDLE},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{GetSecurityInfo, SE_FILE_OBJECT, SE_KERNEL_OBJECT},
        DACL_SECURITY_INFORMATION, GetAce, GetSecurityDescriptorLength, IsValidAcl,
        OWNER_SECURITY_INFORMATION,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ, GetFileInformationByHandle, GetFinalPathNameByHandleW, OPEN_EXISTING,
    },
    System::{Com::CoTaskMemFree, Threading::QueryFullProcessImageNameW},
    UI::Shell::{FOLDERID_ProgramFiles, SHGetKnownFolderPath},
};

const READ_CONTROL: u32 = 0x0002_0000;
const MAX_PATH_UNITS: usize = 32_768;
const MUTATE: u32 = 0x4000_0000 | 0x1000_0000 | 0x000d_0156;
// SYSTEM, local Administrators, Windows Modules Installer. Owners/ACEs outside
// this administrative trust set must not be able to modify an installation.
fn trusted(sid: &str) -> bool {
    matches!(
        sid,
        "S-1-5-18"
            | "S-1-5-32-544"
            | "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464"
    )
}

#[derive(Clone, Copy)]
pub enum InstalledFile {
    Helper,
    Agent,
    WinDivertDll,
    WinDivertDriver,
    Desktop,
    Maintenance,
    StagedMaintenance,
    CaptureReady,
    MaintenanceBlock,
}
impl InstalledFile {
    fn relative(self) -> &'static str {
        match self {
            Self::Helper => "rokbattles-capture-helper.exe",
            Self::Desktop => "rokbattles-desktop.exe",
            Self::Maintenance => r".maintenance\rokbattles-capture-maintenance.exe",
            Self::StagedMaintenance => r".maintenance\rokbattles-capture-maintenance.next.exe",
            Self::CaptureReady => ".capture-ready-v1",
            Self::MaintenanceBlock => ".capture-maintenance",
            Self::Agent => "rokbattles-desktop-agent.exe",
            Self::WinDivertDll => r"capture\windivert\x86_64-pc-windows-msvc\WinDivert.dll",
            Self::WinDivertDriver => r"capture\windivert\x86_64-pc-windows-msvc\WinDivert64.sys",
        }
    }
}

/// Retains no-write/no-delete handles to each protected directory and file for
/// the lifetime of native code use, preventing validation/load replacement races.
pub struct ProtectedInstallation {
    _handles: Vec<OwnedHandle>,
    path: PathBuf,
}
impl ProtectedInstallation {
    pub fn open(file: InstalledFile) -> io::Result<Self> {
        let program_files = program_files()?;
        let root = program_files.join("ROK Battles");
        let path = root.join(file.relative());
        let mut handles = Vec::new();
        handles.push(verify_path(&program_files)?);
        handles.push(verify_path(&root)?);
        let relative = Path::new(file.relative());
        let mut part = root;
        for component in relative.components() {
            part.push(component);
            handles.push(verify_path(&part)?);
        }
        Ok(Self { _handles: handles, path })
    }
    /// Must be called while holding NativeOpen. Presence or uncertainty about
    /// the protected maintenance marker forbids every native capture open.
    pub fn maintenance_ready() -> io::Result<Self> {
        let ready = Self::open(InstalledFile::CaptureReady)?;
        let root = ready.path.parent().ok_or_else(denied)?;
        match std::fs::symlink_metadata(root.join(".capture-maintenance")) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            _ => return Err(denied()),
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&ready.path)?.take(128).read_to_end(&mut bytes)?;
        if bytes != b"ROKBattlesCaptureReady:1\n" {
            return Err(denied());
        }
        Ok(ready)
    }
    /// Separately installed Npcap only. Neither the DLLs nor driver are bundled
    /// or searched in PATH, cwd, a user directory or a caller-selected location.
    pub fn installed_npcap() -> io::Result<(Self, Self)> {
        let system = system_directory()?;
        let root = system.parent().ok_or_else(denied)?;
        let library = system.join("Npcap").join("wpcap.dll");
        let driver = system.join("drivers").join("npcap.sys");
        let mut libraries = Vec::new();
        for path in [
            root.to_path_buf(),
            system.clone(),
            system.join("Npcap"),
            library.clone(),
            system.join("Npcap").join("Packet.dll"),
        ] {
            libraries.push(verify_path(&path)?);
        }
        let mut drivers = Vec::new();
        for path in [root.to_path_buf(), system.clone(), system.join("drivers"), driver.clone()] {
            drivers.push(verify_path(&path)?);
        }
        Ok((Self { _handles: libraries, path: library }, Self { _handles: drivers, path: driver }))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub(super) fn verify_process_image(process: HANDLE, expected: InstalledFile) -> io::Result<()> {
    let installation = ProtectedInstallation::open(expected)?;
    let mut name = vec![0u16; MAX_PATH_UNITS];
    let mut length = name.len() as u32;
    // SAFETY: query-only live process, writable bounded UTF-16 output and size.
    if unsafe { QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &mut length) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let name = name.get(..length as usize).ok_or_else(denied)?;
    let actual = PathBuf::from(std::ffi::OsString::from_wide(name));
    if !same_path(&actual, installation.path()) {
        return Err(denied());
    }
    Ok(())
}

pub(super) fn system_directory() -> io::Result<PathBuf> {
    let mut bytes = vec![0u16; MAX_PATH_UNITS];
    // SAFETY: fixed OS system directory query and bounded writable output.
    let length = unsafe {
        windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
            bytes.as_mut_ptr(),
            bytes.len() as u32,
        )
    } as usize;
    if length == 0 || length >= bytes.len() {
        return Err(denied());
    }
    let path =
        PathBuf::from(std::ffi::OsString::from_wide(bytes.get(..length).ok_or_else(denied)?));
    if !path.is_absolute() {
        return Err(denied());
    }
    Ok(path)
}

fn program_files() -> io::Result<PathBuf> {
    let mut output = ptr::null_mut();
    // SAFETY: fixed OS known-folder GUID; no user token/path/environment input.
    let result =
        unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramFiles, 0, ptr::null_mut(), &mut output) };
    if result < 0 || output.is_null() {
        return Err(denied());
    }
    struct Allocation(*mut u16);
    impl Drop for Allocation {
        fn drop(&mut self) {
            // SAFETY: SHGetKnownFolderPath uses CoTaskMemAlloc.
            unsafe { CoTaskMemFree(self.0.cast()) };
        }
    }
    let allocation = Allocation(output);
    let mut length = 0;
    // SAFETY: OS returns a NUL-terminated allocation; Windows path limit bounds scanning.
    while length < MAX_PATH_UNITS && unsafe { *output.wrapping_add(length) } != 0 {
        length += 1;
    }
    if length == MAX_PATH_UNITS {
        return Err(denied());
    }
    // SAFETY: measured initialized range of the live allocation.
    let path = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
        std::slice::from_raw_parts(output, length)
    }));
    drop(allocation);
    if !path.is_absolute() {
        return Err(denied());
    }
    Ok(path)
}

fn same_path(a: &Path, b: &Path) -> bool {
    // Generated fixed paths are ASCII except the OS Program Files prefix. The
    // Windows ordinal case-insensitive comparison avoids Unicode case folding.
    let a: Vec<u16> = a.as_os_str().encode_wide().collect();
    let b: Vec<u16> = b.as_os_str().encode_wide().collect();
    // SAFETY: two bounded initialized buffers, explicit lengths, no NUL requirement.
    unsafe {
        windows_sys::Win32::Globalization::CompareStringOrdinal(
            a.as_ptr(),
            a.len() as i32,
            b.as_ptr(),
            b.len() as i32,
            1,
        ) == 2
    }
}

fn verify_path(path: &Path) -> io::Result<OwnedHandle> {
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: absolute fixed path; open reparse point itself, deny write/delete
    // sharing, no inheritance, no creation. Works for files and directories.
    let file = own(unsafe {
        CreateFileW(
            name.as_ptr(),
            READ_CONTROL | FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    })?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: live file handle and initialized writable information struct.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(denied());
    }
    let mut final_name = vec![0u16; MAX_PATH_UNITS];
    // SAFETY: live handle and bounded writable path buffer.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            final_name.as_mut_ptr(),
            final_name.len() as u32,
            0,
        )
    } as usize;
    if length == 0 || length >= final_name.len() {
        return Err(denied());
    }
    let final_name =
        PathBuf::from(std::ffi::OsString::from_wide(final_name.get(..length).ok_or_else(denied)?));
    let expected = PathBuf::from(format!(r"\\?\{}", path.display()));
    if !same_path(&final_name, &expected) {
        return Err(denied());
    }
    verify_acl(file.as_raw_handle())?;
    Ok(file)
}

pub fn verify_admin_only_kernel_object(handle: BorrowedHandle<'_>) -> io::Result<()> {
    verify_object_acl(handle.as_raw_handle(), SE_KERNEL_OBJECT, true)
}
pub fn installation_root() -> io::Result<PathBuf> {
    Ok(program_files()?.join("ROK Battles"))
}
pub fn validate_protected_path(path: &Path) -> io::Result<OwnedHandle> {
    let root = installation_root()?;
    if !path.starts_with(&root)
        || path.components().any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(denied());
    }
    let mut current = root.clone();
    let mut opened = verify_path(&current)?;
    for part in path.strip_prefix(&root).map_err(|_error| denied())?.components() {
        current.push(part);
        opened = verify_path(&current)?;
    }
    Ok(opened)
}
fn verify_acl(file: HANDLE) -> io::Result<()> {
    verify_object_acl(file, SE_FILE_OBJECT, false)
}
fn verify_object_acl(file: HANDLE, object: i32, administrative_only: bool) -> io::Result<()> {
    let mut owner = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    // SAFETY: live read-control handle and writable outputs. Returned pointers
    // borrow the separately allocated descriptor retained until function exit.
    let result = unsafe {
        GetSecurityInfo(
            file,
            object,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut dacl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    let _allocation = LocalAllocation(descriptor);
    if owner.is_null() || dacl.is_null() || descriptor.is_null() {
        return Err(denied());
    }
    // SAFETY: successful GetSecurityInfo returns a valid descriptor allocation.
    let descriptor_bytes = unsafe { GetSecurityDescriptorLength(descriptor) } as usize;
    let start = descriptor as usize;
    let end = start.checked_add(descriptor_bytes).ok_or_else(denied)?;
    let acl_start = dacl as usize;
    if acl_start < start || acl_start.checked_add(size_of::<ACL>()).ok_or_else(denied)? > end {
        return Err(denied());
    }
    // SAFETY: ACL header extent checked against the owning descriptor allocation.
    let acl_size = usize::from(unsafe { (*dacl).AclSize });
    let acl_end = acl_start.checked_add(acl_size).ok_or_else(denied)?;
    if acl_size < size_of::<ACL>() || acl_end > end {
        return Err(denied());
    }
    // SAFETY: entire ACL is bounded by the descriptor allocation.
    if unsafe { IsValidAcl(dacl) } == 0 {
        return Err(denied());
    }
    bound_sid(owner, start, end)?;
    let accepted = |sid: &str| {
        if administrative_only { matches!(sid, "S-1-5-18" | "S-1-5-32-544") } else { trusted(sid) }
    };
    if !accepted(&sid_string(owner)?) {
        return Err(denied());
    }
    // SAFETY: GetSecurityInfo supplies a validated ACL within live descriptor.
    for index in 0..unsafe { (*dacl).AceCount } {
        let mut ace: *mut c_void = ptr::null_mut();
        // SAFETY: index bounded by OS-supplied ACL count; writable ACE pointer.
        if unsafe { GetAce(dacl, u32::from(index), &mut ace) } == 0 {
            return Err(denied());
        }
        let ace_start = ace as usize;
        if ace_start < acl_start
            || ace_start.checked_add(size_of::<ACE_HEADER>()).ok_or_else(denied)? > acl_end
        {
            return Err(denied());
        }
        // SAFETY: header extent verified within a validated ACL.
        let header = unsafe { ptr::read_unaligned(ace.cast::<ACE_HEADER>()) };
        let ace_end = ace_start.checked_add(usize::from(header.AceSize)).ok_or_else(denied)?;
        if ace_end > acl_end || usize::from(header.AceSize) < size_of::<ACE_HEADER>() {
            return Err(denied());
        }
        // Ignore inherit-only ACEs and ordinary DENY ACEs. Fail closed on unfamiliar
        // allow/object/callback ACE forms instead of misparsing a writable grant.
        if header.AceFlags & 0x08 != 0 || header.AceType == 1 {
            continue;
        }
        if header.AceType != 0 || usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>() {
            return Err(denied());
        }
        // SAFETY: ordinary allow ACE layout validated above; SID starts inline.
        let mask = unsafe { ptr::read_unaligned(ace.cast::<ACCESS_ALLOWED_ACE>()).Mask };
        // SAFETY: known ACE layout with inline SidStart; validated by GetSecurityInfo.
        let sid = unsafe { ace.cast::<u8>().add(offset_of!(ACCESS_ALLOWED_ACE, SidStart)).cast() };
        bound_sid(sid, ace_start, ace_end)?;
        if (if administrative_only { mask != 0 } else { mask & MUTATE != 0 })
            && !accepted(&sid_string(sid)?)
        {
            return Err(denied());
        }
    }
    Ok(())
}

// Validate SID revision/count byte availability and full subauthority extent
// before IsValidSid/ConvertSidToStringSid can dereference its native pointer.
fn bound_sid(sid: *mut c_void, start: usize, end: usize) -> io::Result<()> {
    let address = sid as usize;
    if address < start || address.checked_add(8).ok_or_else(denied)? > end {
        return Err(denied());
    }
    // SAFETY: first 8 bytes are within the retained owner allocation.
    let count = usize::from(unsafe { *sid.cast::<u8>().wrapping_add(1) });
    if count > 15 || address.checked_add(8 + count * 4).ok_or_else(denied)? > end {
        return Err(denied());
    }
    Ok(())
}
