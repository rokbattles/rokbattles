//! Windows local-only named pipes with logon-session isolation and mutual OS identity checks.
//! No identity is accepted from an IPC request, environment variable or command line.

use crate::SERVICE_NAME;
use std::{
    ffi::c_void,
    io,
    mem::{size_of, size_of_val},
    os::windows::io::{AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions};
use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        GetTokenInformation, IsValidSid, RevertToSelf, SECURITY_ATTRIBUTES, TOKEN_GROUPS,
        TOKEN_QUERY, TOKEN_USER, TokenLogonSid, TokenUser,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_OVERLAPPED, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_WRITE_DATA,
        OPEN_EXISTING, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
    },
    System::{
        Pipes::{
            GetNamedPipeClientProcessId, GetNamedPipeServerProcessId, ImpersonateNamedPipeClient,
        },
        Services::{
            CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
            SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_QUERY_STATUS, SERVICE_RUNNING,
            SERVICE_STATUS_PROCESS,
        },
        Threading::{
            GetCurrentProcess, GetCurrentThread, GetProcessTimes, OpenProcess, OpenProcessToken,
            OpenThreadToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
};
mod trust;
pub use trust::{
    InstalledFile, ProtectedInstallation, installation_root, validate_protected_path,
    verify_admin_only_kernel_object,
};

const SYNCHRONIZE: u32 = 0x0010_0000;
const MAX_TOKEN_BYTES: usize = 65_536;
const MAX_SID_CHARS: usize = 184;

fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture IPC peer identity rejected")
}
fn last_error() -> io::Error {
    io::Error::last_os_error()
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

/// User SID and logon SID copied from a kernel token. Deliberately not serializable.
#[derive(Clone, PartialEq, Eq)]
pub struct Identity {
    user_sid: String,
    logon_sid: String,
}
impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Identity(<redacted>)")
    }
}

impl Identity {
    pub fn current() -> io::Result<Self> {
        // SAFETY: GetCurrentProcess returns a valid non-owning pseudo handle.
        let token = process_token(unsafe { GetCurrentProcess() })?;
        Self::from_token(token.as_handle())
    }

    /// Only handles already obtained from an OS authentication API are accepted.
    pub fn from_token(token: BorrowedHandle<'_>) -> io::Result<Self> {
        let user_sid = token_user(token.as_raw_handle())?;
        let group_buffer = token_information(token.as_raw_handle(), TokenLogonSid)?;
        if group_buffer.len() * size_of::<usize>() < size_of::<TOKEN_GROUPS>() {
            return Err(denied());
        }
        // SAFETY: aligned owned buffer, checked header size, populated by GetTokenInformation.
        let groups = unsafe { &*group_buffer.as_ptr().cast::<TOKEN_GROUPS>() };
        if groups.GroupCount != 1 {
            return Err(denied());
        }
        let group = groups.Groups.first().ok_or_else(denied)?;
        if group.Attributes & 0xc000_0000 != 0xc000_0000 {
            return Err(denied());
        }
        let logon_sid = token_sid(group.Sid, &group_buffer)?;
        if !logon_sid.starts_with("S-1-5-5-") {
            return Err(denied());
        }
        Ok(Self { user_sid, logon_sid })
    }

    pub fn pipe_name(&self) -> String {
        format!(r"\\.\pipe\ROKBattles.Capture.v1.{}", self.logon_sid)
    }
}

use std::os::windows::io::AsHandle;

/// Keeps the process object alive so a recycled numeric PID cannot silently become a new owner.
pub struct ProcessIdentity {
    _process: OwnedHandle,
    identity: Identity,
    creation_time: u64,
}
impl ProcessIdentity {
    pub fn open(pid: u32) -> io::Result<Self> {
        // SAFETY: query-only access, no handle inheritance; returned handle checked below.
        let process =
            own(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid) })?;
        let token = process_token(process.as_raw_handle())?;
        let identity = Identity::from_token(token.as_handle())?;
        let mut times = [windows_sys::Win32::Foundation::FILETIME::default(); 4];
        let [creation, exit, kernel, user] = &mut times;
        // SAFETY: live process handle and four distinct writable FILETIME values.
        if unsafe { GetProcessTimes(process.as_raw_handle(), creation, exit, kernel, user) } == 0 {
            return Err(last_error());
        }
        let creation_time =
            (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime);
        Ok(Self { _process: process, identity, creation_time })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn creation_time(&self) -> u64 {
        self.creation_time
    }
    pub fn is_alive(&self) -> bool {
        // SAFETY: pinned live process handle opened with SYNCHRONIZE; zero timeout.
        unsafe {
            windows_sys::Win32::System::Threading::WaitForSingleObject(
                self._process.as_raw_handle(),
                0,
            ) == windows_sys::Win32::Foundation::WAIT_TIMEOUT
        }
    }
}

/// One instance per logon. A preexisting server causes an error, never a permissive retry.
pub fn create_server(identity: &Identity) -> io::Result<NamedPipeServer> {
    // FILE_CREATE_PIPE_INSTANCE / FILE_APPEND_DATA (0x4) is deliberately absent
    // from the logon ACE. Generic write would let a user create a spoofed instance.
    let sddl = wide(&format!("O:SYG:SYD:P(A;;GA;;;SY)(A;;0x00120183;;;{})", identity.logon_sid));
    let mut descriptor = ptr::null_mut();
    // SAFETY: live NUL-terminated SDDL and writable output. Its only variable is a kernel SID.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(last_error());
    }
    let descriptor = LocalAllocation(descriptor);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // SAFETY: attributes and descriptor remain live until CreateNamedPipe returns;
    // Windows copies the descriptor, and the resulting handle is non-inheritable.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .max_instances(1)
            .in_buffer_size(4096)
            .out_buffer_size(65_547)
            .create_with_security_attributes_raw(
                identity.pipe_name(),
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
            )
    }
}

/// Authenticate after a complete Start request has been read. Impersonation covers
/// only synchronous token queries; no await, filesystem or loader operation occurs.
pub fn authenticate_client(
    server: &NamedPipeServer,
    expected: &Identity,
) -> io::Result<ProcessIdentity> {
    let handle = server.as_raw_handle();
    let mut pid = 0;
    // SAFETY: live pipe and writable u32; API derives PID from the pipe, not a message.
    if unsafe { GetNamedPipeClientProcessId(handle, &mut pid) } == 0 {
        return Err(last_error());
    }
    // SAFETY: Start was read from this connected pipe; identity-only SQOS is accepted.
    if unsafe { ImpersonateNamedPipeClient(handle) } == 0 {
        return Err(last_error());
    }
    let result = (|| {
        let mut token = ptr::null_mut();
        // SAFETY: GetCurrentThread returns a non-owning pseudo handle.
        let thread = unsafe { GetCurrentThread() };
        // SAFETY: query the current thread while impersonating, asking only TOKEN_QUERY.
        if unsafe { OpenThreadToken(thread, TOKEN_QUERY, 1, &mut token) } == 0 {
            return Err(last_error());
        }
        let token = own(token)?;
        Identity::from_token(token.as_handle())
    })();
    // SAFETY: always restore the thread immediately, including token-query failure.
    // Continuing a privileged service after failed restoration would be unsafe.
    if unsafe { RevertToSelf() } == 0 {
        std::process::abort();
    }
    if result? != *expected {
        return Err(denied());
    }
    let process = ProcessIdentity::open(pid)?;
    trust::verify_process_image(process._process.as_raw_handle(), InstalledFile::Agent)?;
    if process.identity() != expected {
        return Err(denied());
    }
    Ok(process)
}

/// Connect to the current logon's fixed pipe and verify its SCM process identity.
/// Does not install, start, elevate or reconfigure the service.
pub fn connect_current_user() -> io::Result<NamedPipeClient> {
    let identity = Identity::current()?;
    let path = wide(&identity.pipe_name());
    // SAFETY: fixed local pipe name; specific rights exclude CREATE_PIPE_INSTANCE.
    // Identification SQOS prevents a spoofing server from using our credentials.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_READ_DATA | FILE_WRITE_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            ptr::null_mut(),
        )
    };
    let handle = own(handle)?;
    authenticate_server(handle.as_raw_handle())?;
    use std::os::windows::io::IntoRawHandle;
    // SAFETY: uniquely owned overlapped named pipe handle; ownership moves to Tokio.
    unsafe { NamedPipeClient::from_raw_handle(handle.into_raw_handle()) }
}

fn authenticate_server(pipe: HANDLE) -> io::Result<()> {
    let mut pid = 0;
    // SAFETY: live pipe and writable PID output.
    if unsafe { GetNamedPipeServerProcessId(pipe, &mut pid) } == 0 {
        return Err(last_error());
    }
    // Pin the process object before checking the SCM PID to prevent PID reuse.
    // SAFETY: query-only process handle; no inherited handles.
    let process =
        own(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE, 0, pid) })?;
    trust::verify_process_image(process.as_raw_handle(), InstalledFile::Helper)?;
    let token = process_token(process.as_raw_handle())?;
    if token_user(token.as_raw_handle())? != "S-1-5-18" {
        return Err(denied());
    }
    // SAFETY: null machine/database selects the local SCM with query-only rights.
    let manager = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CONNECT) };
    if manager.is_null() {
        return Err(last_error());
    }
    let manager = ServiceHandle(manager);
    let name = wide(SERVICE_NAME);
    // SAFETY: fixed service name and live manager; query-only access.
    let service = unsafe { OpenServiceW(manager.0, name.as_ptr(), SERVICE_QUERY_STATUS) };
    if service.is_null() {
        return Err(last_error());
    }
    let service = ServiceHandle(service);
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut needed = 0;
    // SAFETY: correctly sized status buffer, live service and writable size.
    if unsafe {
        QueryServiceStatusEx(
            service.0,
            SC_STATUS_PROCESS_INFO,
            (&mut status as *mut SERVICE_STATUS_PROCESS).cast(),
            size_of_val(&status) as u32,
            &mut needed,
        )
    } == 0
    {
        return Err(last_error());
    }
    if status.dwCurrentState != SERVICE_RUNNING || status.dwProcessId != pid || pid == 0 {
        return Err(denied());
    }
    Ok(())
}

fn process_token(process: HANDLE) -> io::Result<OwnedHandle> {
    let mut token = ptr::null_mut();
    // SAFETY: caller supplies a live process handle; output is initialized and query-only.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(last_error());
    }
    own(token)
}

fn own(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(last_error());
    }
    // SAFETY: every caller transfers a newly returned unique Windows handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn token_information(token: HANDLE, class: i32) -> io::Result<Vec<usize>> {
    let mut length = 0;
    // SAFETY: size-only query with null output; token remains live.
    unsafe { GetTokenInformation(token, class, ptr::null_mut(), 0, &mut length) };
    if last_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
        || length as usize > MAX_TOKEN_BYTES
        || length == 0
    {
        return Err(denied());
    }
    let mut buffer = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
    // SAFETY: aligned output buffer is at least the exact size returned above.
    if unsafe { GetTokenInformation(token, class, buffer.as_mut_ptr().cast(), length, &mut length) }
        == 0
    {
        return Err(last_error());
    }
    Ok(buffer)
}

fn token_user(token: HANDLE) -> io::Result<String> {
    let buffer = token_information(token, TokenUser)?;
    if buffer.len() * size_of::<usize>() < size_of::<TOKEN_USER>() {
        return Err(denied());
    }
    // SAFETY: checked aligned TOKEN_USER buffer returned by the OS; SID borrows it.
    token_sid(unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid }, &buffer)
}

fn token_sid(sid: *mut c_void, buffer: &[usize]) -> io::Result<String> {
    let start = buffer.as_ptr() as usize;
    let end = start.checked_add(std::mem::size_of_val(buffer)).ok_or_else(denied)?;
    let address = sid as usize;
    if address < start || address.checked_add(8).ok_or_else(denied)? > end {
        return Err(denied());
    }
    // SAFETY: fixed SID header is inside the retained token information allocation.
    let count = usize::from(unsafe { *sid.cast::<u8>().wrapping_add(1) });
    if count > 15 || address.checked_add(8 + count * 4).ok_or_else(denied)? > end {
        return Err(denied());
    }
    sid_string(sid)
}

fn sid_string(sid: *mut c_void) -> io::Result<String> {
    // SAFETY: all callers supply a SID embedded in a live OS token information buffer.
    if unsafe { IsValidSid(sid) } == 0 {
        return Err(denied());
    }
    let mut text = ptr::null_mut();
    // SAFETY: validated live SID, writable allocation output owned below.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(last_error());
    }
    let allocation = LocalAllocation(text.cast());
    let mut length = 0;
    // SAFETY: ConvertSidToStringSidW returns a NUL-terminated string allocation.
    while length < MAX_SID_CHARS && unsafe { *text.wrapping_add(length) } != 0 {
        length += 1;
    }
    if length == MAX_SID_CHARS {
        return Err(denied());
    }
    // SAFETY: length measured within the owned UTF-16 string through its terminator.
    let value = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
        .map_err(|_error| denied())?;
    drop(allocation);
    Ok(value)
}

struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: sole owner of an allocation returned by a LocalAlloc-family API.
        unsafe { LocalFree(self.0) };
    }
}
struct ServiceHandle(*mut c_void);
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // SAFETY: sole owner of a live SCM/service handle.
        unsafe { CloseServiceHandle(self.0) };
    }
}
