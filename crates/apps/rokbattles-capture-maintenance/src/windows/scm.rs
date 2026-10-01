use super::{denied, wide};
use std::{
    io,
    mem::size_of,
    path::Path,
    ptr, thread,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_MARKED_FOR_DELETE,
        LocalFree,
    },
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
        DACL_SECURITY_INFORMATION,
    },
    System::Services::*,
};
pub const HELPER: &str = "ROKBattlesCapture";
pub const DRIVER: &str = "WinDivert";
const RIGHTS: u32 = SERVICE_QUERY_CONFIG
    | SERVICE_QUERY_STATUS
    | SERVICE_START
    | SERVICE_STOP
    | SERVICE_CHANGE_CONFIG
    | 0x0004_0000
    | 0x0001_0000;
pub struct Service(SC_HANDLE);
impl Drop for Service {
    fn drop(&mut self) {
        // SAFETY: owns one SCM/service handle.
        unsafe { CloseServiceHandle(self.0) };
    }
}
fn error() -> io::Error {
    let e = io::Error::last_os_error();
    if e.raw_os_error() == Some(ERROR_SERVICE_MARKED_FOR_DELETE as i32) {
        io::Error::new(io::ErrorKind::WouldBlock, "service deletion requires restart")
    } else {
        e
    }
}
fn manager() -> io::Result<Service> {
    // SAFETY: local SCM and specific provisioning rights.
    let h = unsafe {
        OpenSCManagerW(ptr::null(), ptr::null(), SC_MANAGER_CONNECT | SC_MANAGER_CREATE_SERVICE)
    };
    if h.is_null() { Err(error()) } else { Ok(Service(h)) }
}
fn open(name: &str) -> io::Result<Option<Service>> {
    let scm = manager()?;
    let name = wide(name);
    // SAFETY: fixed name and live local manager, output checked.
    let h = unsafe { OpenServiceW(scm.0, name.as_ptr(), RIGHTS) };
    if h.is_null() {
        let e = error();
        if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
            Ok(None)
        } else {
            Err(e)
        }
    } else {
        Ok(Some(Service(h)))
    }
}
fn status(service: &Service) -> io::Result<SERVICE_STATUS_PROCESS> {
    let mut s = SERVICE_STATUS_PROCESS::default();
    let mut n = 0;
    // SAFETY: live query handle and correct output byte length.
    if unsafe {
        QueryServiceStatusEx(
            service.0,
            SC_STATUS_PROCESS_INFO,
            (&mut s as *mut SERVICE_STATUS_PROCESS).cast(),
            size_of::<SERVICE_STATUS_PROCESS>() as u32,
            &mut n,
        )
    } == 0
    {
        return Err(error());
    }
    Ok(s)
}
fn wait(service: &Service, expected: u32) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let s = status(service)?;
        if s.dwCurrentState == expected {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "service did not quiesce; restart required",
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
}
pub fn verify(name: &str, root: &Path) -> io::Result<()> {
    let Some(service) = open(name)? else {
        return Ok(());
    };
    let mut needed = 0;
    // SAFETY: size-only configuration query.
    unsafe { QueryServiceConfigW(service.0, ptr::null_mut(), 0, &mut needed) };
    if io::Error::last_os_error().raw_os_error() != Some(ERROR_INSUFFICIENT_BUFFER as i32)
        || !(size_of::<QUERY_SERVICE_CONFIGW>()..=16_384).contains(&(needed as usize))
    {
        return Err(denied());
    }
    let mut words = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
    // SAFETY: aligned allocation at least the queried byte size.
    if unsafe { QueryServiceConfigW(service.0, words.as_mut_ptr().cast(), needed, &mut needed) }
        == 0
    {
        return Err(error());
    }
    // SAFETY: OS query returned a complete configuration header in aligned storage.
    let config = unsafe { &*words.as_ptr().cast::<QUERY_SERVICE_CONFIGW>() };
    let path = string(&words, config.lpBinaryPathName)?;
    let expected = image(name, root);
    let path = if name == DRIVER { path.strip_prefix(r"\??\").unwrap_or(&path) } else { &path };
    if !path.eq_ignore_ascii_case(&expected) || config.dwServiceType != kind(name) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "another installation owns capture service",
        ));
    }
    if name == HELPER
        && !string(&words, config.lpServiceStartName)?.eq_ignore_ascii_case("LocalSystem")
    {
        return Err(denied());
    }
    Ok(())
}
fn string(words: &[usize], p: *const u16) -> io::Result<String> {
    let start = words.as_ptr() as usize;
    let end = start + std::mem::size_of_val(words);
    let address = p as usize;
    if address < start || address >= end || !address.is_multiple_of(2) {
        return Err(denied());
    }
    // SAFETY: pointer and maximum extent are within retained query allocation.
    let chars = unsafe { std::slice::from_raw_parts(p, (end - address) / 2) };
    let len = chars.iter().position(|x| *x == 0).ok_or_else(denied)?;
    String::from_utf16(chars.get(..len).ok_or_else(denied)?).map_err(|_error| denied())
}
fn kind(name: &str) -> u32 {
    if name == DRIVER { SERVICE_KERNEL_DRIVER } else { SERVICE_WIN32_OWN_PROCESS }
}
fn image(name: &str, root: &Path) -> String {
    if name == DRIVER {
        root.join(r"capture\windivert\x86_64-pc-windows-msvc\WinDivert64.sys")
            .to_string_lossy()
            .into_owned()
    } else {
        format!("\"{}\" --service", root.join("rokbattles-capture-helper.exe").display())
    }
}
pub fn stop(name: &str) -> io::Result<()> {
    let Some(service) = open(name)? else {
        return Ok(());
    };
    // Disable before stop: SCM failure recovery/dependencies cannot reactivate a
    // half-updated image after this maintenance process exits unexpectedly.
    // SAFETY: retain all configuration except start mode on our already verified service.
    if unsafe {
        ChangeServiceConfigW(
            service.0,
            SERVICE_NO_CHANGE,
            SERVICE_DISABLED,
            SERVICE_NO_CHANGE,
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
        )
    } == 0
    {
        return Err(error());
    }
    let s = status(&service)?;
    if s.dwCurrentState == SERVICE_STOPPED {
        return Ok(());
    }
    if s.dwCurrentState != SERVICE_STOP_PENDING {
        let mut out = SERVICE_STATUS::default();
        // SAFETY: fixed STOP control and writable status; never force kills another app.
        if unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut out) } == 0 {
            return Err(error());
        }
    }
    wait(&service, SERVICE_STOPPED)
}
pub fn register(name: &str, root: &Path) -> io::Result<()> {
    verify(name, root)?;
    let image = wide(&image(name, root));
    let service_name = wide(name);
    let display = wide(if name == HELPER { "ROK Battles Capture" } else { "WinDivert" });
    let dependency = if name == HELPER && cfg!(target_arch = "x86_64") {
        wide("WinDivert\0")
    } else {
        vec![0, 0]
    };
    let service = if let Some(service) = open(name)? {
        // SAFETY: verified fixed service; fixed image, SYSTEM account and dependency only.
        if unsafe {
            ChangeServiceConfigW(
                service.0,
                kind(name),
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                image.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                dependency.as_ptr(),
                ptr::null(),
                ptr::null(),
                display.as_ptr(),
            )
        } == 0
        {
            return Err(error());
        }
        service
    } else {
        let manager = manager()?;
        // SAFETY: all pointers are terminated owned fixed data; null account is SYSTEM.
        let handle = unsafe {
            CreateServiceW(
                manager.0,
                service_name.as_ptr(),
                display.as_ptr(),
                RIGHTS,
                kind(name),
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                image.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                dependency.as_ptr(),
                ptr::null(),
                ptr::null(),
            )
        };
        if handle.is_null() {
            return Err(error());
        }
        Service(handle)
    };
    let sddl = wide("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;CCLCSWLOCRRC;;;BU)");
    let mut sd = ptr::null_mut();
    // SAFETY: fixed admin-write/user-query service descriptor; initialized output.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut sd,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(error());
    }
    // SAFETY: service has WRITE_DAC and descriptor remains allocated through the call.
    let ok = unsafe { SetServiceObjectSecurity(service.0, DACL_SECURITY_INFORMATION, sd) };
    // SAFETY: descriptor returned by LocalAlloc API.
    unsafe { LocalFree(sd) };
    if ok == 0 {
        return Err(error());
    }
    Ok(())
}
pub fn start(name: &str) -> io::Result<()> {
    let service = open(name)?.ok_or_else(denied)?;
    if status(&service)?.dwCurrentState != SERVICE_RUNNING {
        // SAFETY: fixed service and no runtime arguments.
        if unsafe { StartServiceW(service.0, 0, ptr::null()) } == 0 {
            return Err(error());
        }
    }
    wait(&service, SERVICE_RUNNING)
}
pub fn remove(name: &str) -> io::Result<()> {
    if let Some(service) = open(name)? {
        if status(&service)?.dwCurrentState != SERVICE_STOPPED {
            return Err(denied());
        }
        // SAFETY: already verified and stopped owned service. No unrelated service name.
        if unsafe { DeleteService(service.0) } == 0 {
            return Err(error());
        }
        drop(service);
        // A retained external SCM handle may keep deletion pending; the caller must
        // not report success or remove payloads in that state.
        if open(name)?.is_some() {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "service deletion pending"));
        }
    }
    Ok(())
}
