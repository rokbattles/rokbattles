//! Shared with explicit maintenance provisioning. Runtime never installs/starts a driver.
use std::{
    io,
    marker::PhantomData,
    mem::size_of,
    os::windows::io::{AsHandle, AsRawHandle, FromRawHandle, OwnedHandle},
    ptr,
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::{LocalFree, WAIT_ABANDONED, WAIT_OBJECT_0},
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    System::Threading::{CreateMutexExW, MUTEX_MODIFY_STATE, ReleaseMutex, WaitForSingleObject},
};
pub const NAME: &str = r"Global\ROKBattles.Capture.NativeOpen.v1";
/// Windows mutex ownership is thread-affine, including release during Drop.
///
/// ```compile_fail
/// use rokbattles_capture_helper::windows::open_lock::OpenLock;
/// fn require_send<T: Send>() {}
/// require_send::<OpenLock>();
/// ```
pub struct OpenLock(OwnedHandle, PhantomData<Rc<()>>);
impl OpenLock {
    pub fn acquire() -> io::Result<Self> {
        Self::acquire_named(NAME, 5_000)
    }
    fn acquire_named(name: &str, timeout_ms: u32) -> io::Result<Self> {
        // Administrators is an assignable owner for both the elevated installer
        // and SYSTEM. Do not depend on the machine's Object Creator policy.
        let sddl: Vec<u16> =
            "O:BAD:P(A;;GA;;;SY)(A;;GA;;;BA)".encode_utf16().chain(Some(0)).collect();
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
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
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
        let result = unsafe { WaitForSingleObject(handle.as_raw_handle(), timeout_ms) };
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "capture maintenance is active"));
        }
        Ok(Self(handle, PhantomData))
    }
}
impl Drop for OpenLock {
    fn drop(&mut self) {
        // SAFETY: held by the acquiring thread for its complete lexical lifetime.
        if unsafe { ReleaseMutex(self.0.as_raw_handle()) } == 0 {
            std::process::abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    #[test]
    fn elevated_admin_owner_mutex_verifies_and_excludes_other_threads() {
        // Ephemeral local-namespace object only: no service, driver, capture,
        // installation or persistent Windows security setting is touched.
        let name = format!(
            r"Local\ROKBattles.Capture.NativeOpen.Test.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let lock = OpenLock::acquire_named(&name, 0)
            .expect("administrative CI token can create O:BA mutex");
        let other = name.clone();
        let blocked = std::thread::spawn(move || match OpenLock::acquire_named(&other, 0) {
            Ok(_unexpected) => false,
            Err(error) => error.kind() == io::ErrorKind::WouldBlock,
        })
        .join()
        .expect("mutex contender");
        assert!(blocked, "existing protected mutex must serialize different threads");
        drop(lock);
        let reacquired = OpenLock::acquire_named(&name, 0).expect("released owner can reacquire");
        drop(reacquired);
    }
}
