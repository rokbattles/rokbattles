use std::{
    ffi::{CStr, c_char, c_int, c_void},
    path::Path,
    ptr::NonNull,
};

use libloading::Library;

use crate::{Error, Receive, SNAPLEN, library, packet};

const FILTER: &CStr = c"inbound and tcp.SrcPort == 3101 and tcp.DstPort != 3101";
const NETWORK: c_int = 0;
const SNIFF: u64 = 0x0001;
const RECV_ONLY: u64 = 0x0004;
const NO_INSTALL: u64 = 0x0010;
const FLAGS: u64 = SNIFF | RECV_ONLY | NO_INSTALL;
const SHUTDOWN_RECV: c_int = 1;

// WinDivert 2.2 windivert.h: 8-byte timestamp, two 32-bit flag/reserved
// words and a 64-byte union. Win32's C ABI requires 8-byte struct alignment.
#[repr(C, align(8))]
struct Address {
    timestamp: i64,
    flags: u32,
    reserved: u32,
    data: [u8; 64],
}

const _: () = assert!(std::mem::size_of::<Address>() == 80);
const _: () = assert!(std::mem::align_of::<Address>() == 8);

// WinDivert's exports are C (__cdecl), NOT WINAPI (__stdcall) on x86.
// Choosing extern "system" would silently work on x64 but corrupt x86 calls.
type Open = unsafe extern "C" fn(*const c_char, c_int, i16, u64) -> *mut c_void;
type Recv = unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut u32, *mut Address) -> c_int;
type Shutdown = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type Close = unsafe extern "C" fn(*mut c_void) -> c_int;

struct Api {
    open: Open,
    recv: Recv,
    shutdown: Shutdown,
    close: Close,
    // Function pointers remain private and cannot outlive this retained DLL.
    _library: Option<Library>,
}

/// Loaded functions, without an open driver handle.
pub struct WinDivert {
    api: Api,
}

impl WinDivert {
    /// Load a trusted, architecture-matching WinDivert 2.x DLL from an absolute
    /// path. Dependencies are restricted to System32. No driver is opened.
    ///
    /// # Safety
    /// The caller must establish that the DLL is trusted native code implementing
    /// the WinDivert 2.x ABI. Its initializers run during loading. Use a protected
    /// installation directory; canonicalization does not establish file trust.
    pub unsafe fn load(path: &Path) -> Result<Self, Error> {
        // SAFETY: caller guarantees the DLL's trust and version/architecture ABI.
        let library = unsafe { library::load(path) }?;
        macro_rules! symbol {
            ($name:expr) => {{
                // SAFETY: exact documented C signature, inferred from the field;
                // the resulting private pointer is paired with its live library.
                unsafe { library::symbol(&library, $name) }?
            }};
        }
        let api = Api {
            open: symbol!(c"WinDivertOpen"),
            recv: symbol!(c"WinDivertRecv"),
            shutdown: symbol!(c"WinDivertShutdown"),
            close: symbol!(c"WinDivertClose"),
            _library: Some(library),
        };
        Ok(Self { api })
    }

    /// Explicitly open a passive, inbound, receive-only handle. NO_INSTALL makes
    /// an absent driver an error; this adapter never installs or starts a driver
    /// service itself and never requests elevation or alters security settings.
    pub fn open(&self) -> Result<Capture<'_>, Error> {
        // SAFETY: trusted ABI, valid static NUL-terminated filter, fixed layer,
        // priority and flags. SNIFF leaves the original network packet untouched.
        let handle = unsafe { (self.api.open)(FILTER.as_ptr(), NETWORK, 0, FLAGS) };
        if handle as isize == -1 {
            return Err(last_error("WinDivertOpen"));
        }
        let handle =
            NonNull::new(handle).ok_or(Error::InvalidPacket("WinDivertOpen returned NULL"))?;
        Ok(Capture { api: &self.api, handle })
    }
}

/// One handle, borrowing its DLL owner. All returned packets are owned copies.
pub struct Capture<'a> {
    api: &'a Api,
    handle: NonNull<c_void>,
}

// SAFETY: WinDivert supports concurrent operations on a handle. Receives use
// separate per-call buffers; Shutdown may interrupt a blocking receive. Shared
// references keep Drop from closing the handle while an operation is in flight.
unsafe impl Send for Capture<'_> {}
// SAFETY: as above; no Rust mutable state or borrowed native buffers are shared.
unsafe impl Sync for Capture<'_> {}

impl Capture<'_> {
    /// Blocks waiting for one packet. Call on a capture thread, never a UI/event
    /// loop. Another thread may call `shutdown` through a shared reference to
    /// unblock it. Shutdown drains queued packets before returning `End`.
    pub fn receive(&self) -> Result<Receive, Error> {
        let mut bytes = vec![0; SNAPLEN];
        let mut length = 0;
        let mut address = Address { timestamp: 0, flags: 0, reserved: 0, data: [0; 64] };
        // SAFETY: all outputs are initialized, correctly aligned and sized. This
        // live handle and its library outlive the call. No buffer is reused.
        let result = unsafe {
            (self.api.recv)(
                self.handle.as_ptr(),
                bytes.as_mut_ptr().cast(),
                SNAPLEN as u32,
                &raw mut length,
                &raw mut address,
            )
        };
        if result == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(232) {
                // ERROR_NO_DATA after shutdown
                return Ok(Receive::End);
            }
            return Err(windows_error("WinDivertRecv", error));
        }
        let length = usize::try_from(length)
            .map_err(|_error| Error::InvalidPacket("WinDivert length overflow"))?;
        let bytes =
            bytes.get(..length).ok_or(Error::InvalidPacket("WinDivert length exceeds buffer"))?;
        // Layer/event are NETWORK/PACKET (zero), sniffed bit set, outbound bit
        // clear. Discard ambiguous metadata even if the native filter matched.
        if address.flags & 0xffff != 0
            || address.flags & (1 << 16) == 0
            || address.flags & (1 << 17) != 0
        {
            return Ok(Receive::Discarded);
        }
        Ok(packet::server_packet(bytes)
            .map_or(Receive::Discarded, |ip| Receive::Packet(ip.to_vec())))
    }

    /// Stop future receives and wake blocking readers. Does not inject packets.
    pub fn shutdown(&self) -> Result<(), Error> {
        // SAFETY: live WinDivert handle; documented receive-only shutdown enum.
        let result = unsafe { (self.api.shutdown)(self.handle.as_ptr(), SHUTDOWN_RECV) };
        if result == 0 { Err(last_error("WinDivertShutdown")) } else { Ok(()) }
    }
}

impl Drop for Capture<'_> {
    fn drop(&mut self) {
        // SAFETY: unique owner closes exactly once, before its DLL borrow ends.
        // Rust excludes any in-flight operation while this owner is dropped.
        unsafe { (self.api.close)(self.handle.as_ptr()) };
    }
}

fn last_error(operation: &'static str) -> Error {
    windows_error(operation, std::io::Error::last_os_error())
}

fn windows_error(operation: &'static str, error: std::io::Error) -> Error {
    match error.raw_os_error() {
        Some(5) => Error::PermissionDenied { backend: "WinDivert", detail: error.to_string() },
        Some(code @ (2 | 577 | 654 | 1060 | 1275 | 1753)) => Error::DriverUnavailable { code },
        _ => Error::Native { operation, detail: error.to_string() },
    }
}
