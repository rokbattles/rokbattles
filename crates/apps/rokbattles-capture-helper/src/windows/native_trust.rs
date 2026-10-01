//! Exact upstream payload pins and driver-specific Windows trust policy.
//! No installation, service start, network download or user-selected path.
use rokbattles_capture_ipc::windows::{InstalledFile, ProtectedInstallation};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::AsRawHandle},
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, INVALID_HANDLE_VALUE},
    Security::WinTrust::*,
    System::Services::*,
};

const DLL_BYTES: u64 = 47_616;
const DLL_SHA256: &str = "c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2";
const DRIVER_BYTES: u64 = 94_144;
const DRIVER_SHA256: &str = "8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2";
const SIGNATURE_INDEX: u32 = 1;
fn denied() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "capture payload trust unavailable")
}

pub struct TrustedWinDivert {
    dll: ProtectedInstallation,
    _driver: ProtectedInstallation,
}
impl TrustedWinDivert {
    pub fn verify() -> io::Result<Self> {
        let files = Self::verify_files_cached()?;
        require_running_driver(files._driver.path())?;
        Ok(files)
    }
    /// Shared with explicit maintenance after its online trust/cache preparation.
    /// Only fixed final paths are accepted; no driver service is changed or started.
    pub fn verify_files_cached() -> io::Result<Self> {
        let dll = ProtectedInstallation::open(InstalledFile::WinDivertDll)?;
        let driver = ProtectedInstallation::open(InstalledFile::WinDivertDriver)?;
        verify_hash(dll.path(), DLL_BYTES, DLL_SHA256)?;
        verify_hash(driver.path(), DRIVER_BYTES, DRIVER_SHA256)?;
        verify_driver_signature(driver.path())?;
        Ok(Self { dll, _driver: driver })
    }
    pub fn dll_path(&self) -> &Path {
        self.dll.path()
    }
}

fn verify_hash(path: &Path, length: u64, expected: &str) -> io::Result<()> {
    let file = File::open(path)?;
    if file.metadata()?.len() != length {
        return Err(denied());
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(length + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != length || format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(denied());
    }
    Ok(())
}

fn verify_driver_signature(path: &Path) -> io::Result<()> {
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let file = File::open(path)?;
    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: name.as_ptr(),
        hFile: file.as_raw_handle(),
        pgKnownSubject: ptr::null_mut(),
    };
    let mut signatures = WINTRUST_SIGNATURE_SETTINGS {
        cbStruct: size_of::<WINTRUST_SIGNATURE_SETTINGS>() as u32,
        dwIndex: SIGNATURE_INDEX,
        dwFlags: WSS_VERIFY_SPECIFIC,
        dwVerifiedSigIndex: u32::MAX,
        ..Default::default()
    };
    let mut data = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file_info },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT
            | WTD_DISABLE_MD2_MD4
            | WTD_CACHE_ONLY_URL_RETRIEVAL,
        pSignatureSettings: &mut signatures,
        ..Default::default()
    };
    let mut action = DRIVER_ACTION_VERIFY;
    // SAFETY: documented driver policy with valid file/signature structs, retained
    // immutable file, no UI and explicit secondary signature. No generic fallback.
    let status = unsafe {
        WinVerifyTrust(INVALID_HANDLE_VALUE, &mut action, (&mut data as *mut WINTRUST_DATA).cast())
    };
    data.dwStateAction = WTD_STATEACTION_CLOSE;
    // SAFETY: CLOSE pairs every VERIFY including failed verification, same structs.
    unsafe {
        WinVerifyTrust(INVALID_HANDLE_VALUE, &mut action, (&mut data as *mut WINTRUST_DATA).cast())
    };
    // LONG return: only zero is success (not HRESULT's >=0 convention).
    if status != 0 || signatures.dwVerifiedSigIndex != SIGNATURE_INDEX {
        return Err(denied());
    }
    Ok(())
}

struct Service(SC_HANDLE);
impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: owns one service/SCM handle.
        unsafe { CloseServiceHandle(self.0) };
    }
}
fn require_running_driver(path: &Path) -> io::Result<()> {
    // SAFETY: local SCM, query-only access.
    let scm = unsafe { OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CONNECT) };
    if scm.is_null() {
        return Err(denied());
    }
    let scm = Service(scm);
    let name: Vec<u16> = "WinDivert".encode_utf16().chain(Some(0)).collect();
    // SAFETY: fixed upstream service name, no create/start/change rights.
    let driver =
        unsafe { OpenServiceW(scm.0, name.as_ptr(), SERVICE_QUERY_CONFIG | SERVICE_QUERY_STATUS) };
    if driver.is_null() {
        return Err(denied());
    }
    let driver = Service(driver);
    let mut status = SERVICE_STATUS::default();
    // SAFETY: valid query handle and output struct.
    if unsafe { QueryServiceStatus(driver.0, &mut status) } == 0
        || status.dwCurrentState != SERVICE_RUNNING
        || status.dwServiceType != SERVICE_KERNEL_DRIVER
    {
        return Err(denied());
    }
    let mut size = 0;
    // SAFETY: size-only query for the fixed driver service.
    unsafe { QueryServiceConfigW(driver.0, ptr::null_mut(), 0, &mut size) };
    if io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
        || !(size_of::<QUERY_SERVICE_CONFIGW>()..=16_384).contains(&(size as usize))
    {
        return Err(denied());
    }
    let mut bytes = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    // SAFETY: aligned output allocation at least the returned size.
    if unsafe { QueryServiceConfigW(driver.0, bytes.as_mut_ptr().cast(), size, &mut size) } == 0 {
        return Err(denied());
    }
    // SAFETY: output contains the verified fixed-size QUERY_SERVICE_CONFIGW header.
    let config = unsafe { &*bytes.as_ptr().cast::<QUERY_SERVICE_CONFIGW>() };
    let start = bytes.as_ptr() as usize;
    let end = start + bytes.len() * size_of::<usize>();
    let pointer = config.lpBinaryPathName as usize;
    if pointer < start || pointer >= end || !pointer.is_multiple_of(2) {
        return Err(denied());
    }
    // SAFETY: string pointer extent bounded by the query output allocation.
    let units = unsafe { std::slice::from_raw_parts(config.lpBinaryPathName, (end - pointer) / 2) };
    let length = units.iter().position(|u| *u == 0).ok_or_else(denied)?;
    let actual =
        String::from_utf16(units.get(..length).ok_or_else(denied)?).map_err(|_error| denied())?;
    // Driver ImagePath commonly has a \??\ DOS-device prefix. No environment
    // expansion, arguments, relative paths or alternative service image allowed.
    let actual = actual.strip_prefix(r"\??\").unwrap_or(&actual);
    if !actual.eq_ignore_ascii_case(&path.to_string_lossy()) {
        return Err(denied());
    }
    Ok(())
}
