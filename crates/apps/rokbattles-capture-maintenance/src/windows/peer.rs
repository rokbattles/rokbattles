use super::{denied, own};
use rokbattles_capture_ipc::windows::{Identity, InstalledFile, ProtectedInstallation};
use std::{
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStringExt,
        io::{AsHandle, AsRawHandle, OwnedHandle},
    },
    path::PathBuf,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{FILETIME, HANDLE, WAIT_TIMEOUT},
    Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
    System::{Diagnostics::ToolHelp::*, Threading::*},
};

pub struct Process {
    pub pid: u32,
    handle: OwnedHandle,
    created: u64,
    identity: Identity,
}
impl Process {
    pub fn open(pid: u32) -> io::Result<Self> {
        // SAFETY: query/synchronize only, no handle inheritance.
        let handle =
            own(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | 0x0010_0000, 0, pid) })?;
        let mut token = ptr::null_mut();
        // SAFETY: live process and initialized token output.
        if unsafe { OpenProcessToken(handle.as_raw_handle(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let token = own(token)?;
        let mut elevation = TOKEN_ELEVATION::default();
        let mut needed = 0;
        // SAFETY: typed writable output with matching buffer size and query-only token.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                size_of::<TOKEN_ELEVATION>() as u32,
                &mut needed,
            )
        } == 0
            || elevation.TokenIsElevated == 0
        {
            return Err(denied());
        }
        let identity = Identity::from_token(token.as_handle())?;
        let [mut created, mut exit, mut kernel, mut user] = [FILETIME::default(); 4];
        // SAFETY: live process and distinct writable timestamp outputs.
        if unsafe {
            GetProcessTimes(handle.as_raw_handle(), &mut created, &mut exit, &mut kernel, &mut user)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let created = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
        Ok(Self { pid, handle, created, identity })
    }
    pub fn alive(&self) -> bool {
        // SAFETY: pinned synchronization handle; no blocking wait.
        unsafe { WaitForSingleObject(self.handle.as_raw_handle(), 0) == WAIT_TIMEOUT }
    }
    pub fn same(&self, other: &Self) -> bool {
        self.pid == other.pid
            && self.created == other.created
            && self.identity == other.identity
            && self.alive()
            && other.alive()
    }
    pub fn maintenance(&self) -> io::Result<ProtectedInstallation> {
        let image = ProtectedInstallation::open(InstalledFile::Maintenance)?;
        if !super::same_path(&image_path(self.handle.as_raw_handle())?, image.path()) {
            return Err(denied());
        }
        Ok(image)
    }
    pub fn child_of(&self, parent: &Self) -> io::Result<()> {
        let current_parent = Self::open(parent_pid(self.pid)?)?;
        if !current_parent.same(parent)
            || self.created < parent.created
            || self.identity != parent.identity
        {
            return Err(denied());
        }
        Ok(())
    }
}
pub fn current_parent() -> io::Result<Process> {
    // SAFETY: query only OS current process identifier.
    let current = Process::open(unsafe { GetCurrentProcessId() })?;
    let parent = Process::open(parent_pid(current.pid)?)?;
    current.child_of(&parent)?;
    Ok(parent)
}
pub fn parent_pid(pid: u32) -> io::Result<u32> {
    for entry in processes()? {
        if entry.th32ProcessID == pid {
            return Ok(entry.th32ParentProcessID);
        }
    }
    Err(denied())
}
pub fn processes() -> io::Result<Vec<PROCESSENTRY32W>> {
    // SAFETY: bounded local process inventory; handle uniquely owned.
    let snapshot = own(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })?;
    let mut entry =
        PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    // SAFETY: correctly sized initialized process entry output.
    if unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut result = Vec::new();
    loop {
        result.push(entry);
        if result.len() > 65_536 {
            return Err(denied());
        }
        // SAFETY: live snapshot and reused correctly sized output.
        if unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) } == 0 {
            break;
        }
    }
    Ok(result)
}
pub fn image_path(handle: HANDLE) -> io::Result<PathBuf> {
    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    // SAFETY: query-only process and bounded UTF-16 buffer with writable length.
    if unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut length) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(PathBuf::from(std::ffi::OsString::from_wide(
        buffer.get(..length as usize).ok_or_else(denied)?,
    )))
}
