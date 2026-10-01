use super::denied;
use std::{
    fs::File,
    io,
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::AsRawHandle},
    path::Path,
    ptr,
};
use windows_sys::Win32::{Foundation::INVALID_HANDLE_VALUE, Security::WinTrust::*};
const SIGNATURE_INDEX: u32 = 1;
const ONLINE_REVOCATION: u32 = WTD_REVOKE_WHOLECHAIN;
const ONLINE_FLAGS: u32 = WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT | WTD_DISABLE_MD2_MD4;
pub fn verify_driver_online(path: &Path) -> io::Result<()> {
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
        fdwRevocationChecks: ONLINE_REVOCATION,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file_info },
        dwStateAction: WTD_STATEACTION_VERIFY,
        dwProvFlags: ONLINE_FLAGS,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maintenance_requires_online_whole_chain_revocation() {
        assert_eq!(ONLINE_FLAGS, WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT | WTD_DISABLE_MD2_MD4);
        assert_eq!(ONLINE_FLAGS & WTD_CACHE_ONLY_URL_RETRIEVAL, 0);
        assert_eq!(ONLINE_FLAGS & WTD_REVOCATION_CHECK_NONE, 0);
        assert_eq!(ONLINE_REVOCATION, WTD_REVOKE_WHOLECHAIN);
        assert_eq!(SIGNATURE_INDEX, 1);
    }
}
