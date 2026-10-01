//! Shared with explicit maintenance provisioning. Runtime never installs/starts a driver.
use std::{
    io,
    mem::size_of,
    os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
};
use windows_sys::Win32::{
    Foundation::{LocalFree, WAIT_ABANDONED, WAIT_OBJECT_0},
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    System::Threading::{CreateMutexExW, MUTEX_MODIFY_STATE, ReleaseMutex, WaitForSingleObject},
};
pub const NAME: &str = r"Global\ROKBattles.Capture.NativeOpen.v1";
pub struct OpenLock(OwnedHandle);
impl OpenLock {
    pub fn acquire() -> io::Result<Self> {
        let sddl: Vec<u16> =
            "O:SYG:SYD:P(A;;GA;;;SY)(A;;GA;;;BA)".encode_utf16().chain(Some(0)).collect();
        let mut descriptor = ptr::null_mut();
        // SAFETY: static NUL-terminated SDDL and initialized writable output.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let name: Vec<u16> = NAME.encode_utf16().chain(Some(0)).collect();
        // SAFETY: fixed global name and live protected descriptor, non-inheritable.
        let handle = unsafe {
            CreateMutexExW(
                &attributes,
                name.as_ptr(),
                0,
                0x0010_0000 | 0x0002_0000 | MUTEX_MODIFY_STATE,
            )
        };
        // SAFETY: ConvertStringSecurityDescriptor returned a LocalAlloc allocation;
        // CreateMutex copied it before returning.
        unsafe { LocalFree(descriptor) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: new unique handle from CreateMutexExW; RAII closes it exactly once.
        let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
        rokbattles_capture_ipc::windows::verify_admin_only_kernel_object(handle.as_handle())?;
        // SAFETY: live synchronization handle; bounded wait, no infinite installation lock.
        let result = unsafe { WaitForSingleObject(handle.as_raw_handle(), 5_000) };
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "capture maintenance is active"));
        }
        Ok(Self(handle))
    }
}
impl Drop for OpenLock {
    fn drop(&mut self) {
        // SAFETY: held by the acquiring thread for its complete lexical lifetime.
        unsafe { ReleaseMutex(self.0.as_raw_handle()) };
    }
}
