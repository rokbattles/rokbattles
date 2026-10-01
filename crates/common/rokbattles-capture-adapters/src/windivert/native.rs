use std::{
    ffi::{CStr, c_char, c_int, c_void},
    path::Path,
    ptr::NonNull,
};

use libloading::Library;

use crate::{Error, Receive, SNAPLEN, library, packet};

const FILTER: &CStr = c"inbound and (tcp.SrcPort == 3101 or tcp.SrcPort == 5222) and tcp.DstPort != 3101 and tcp.DstPort != 5222";
const NETWORK: c_int = 0;
const SNIFF: u64 = 0x0001;
const RECV_ONLY: u64 = 0x0004;
const NO_INSTALL: u64 = 0x0010;
const FLAGS: u64 = SNIFF | RECV_ONLY | NO_INSTALL;
const SHUTDOWN_RECV: c_int = 1;

// WinDivert 2.2 windivert.h: 8-byte timestamp, two 32-bit flag/reserved
// words and a 64-byte union. The native ABI requires 8-byte struct alignment.
#[repr(C, align(8))]
struct Address {
    timestamp: i64,
    flags: u32,
    reserved: u32,
    data: [u8; 64],
}

const _: () = assert!(std::mem::size_of::<Address>() == 80);
const _: () = assert!(std::mem::align_of::<Address>() == 8);

// WinDivert exports use the C ABI. This adapter supports Windows x64 only.
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
    /// path. Dependencies are restricted to its trusted directory and System32.
    /// No driver is opened.
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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, ptr};

    use super::*;

    struct State {
        flags: u32,
        bytes: Vec<u8>,
        length: Option<u32>,
        open_result: isize,
        recv_result: c_int,
        closed: usize,
        shutdown: usize,
    }

    impl Default for State {
        fn default() -> Self {
            Self {
                flags: 1 << 16,
                bytes: packet::tests::ipv4(),
                length: None,
                open_result: 1,
                recv_result: 1,
                closed: 0,
                shutdown: 0,
            }
        }
    }

    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

    unsafe extern "C" fn open(
        filter: *const c_char,
        layer: c_int,
        priority: i16,
        flags: u64,
    ) -> *mut c_void {
        // SAFETY: production open provides this valid static C string.
        let filter = unsafe { CStr::from_ptr(filter) };
        assert_eq!(
            filter,
            c"inbound and (tcp.SrcPort == 3101 or tcp.SrcPort == 5222) and tcp.DstPort != 3101 and tcp.DstPort != 5222"
        );
        assert_eq!(layer, NETWORK);
        assert_eq!(priority, 0);
        assert_eq!(flags, 0x15); // SNIFF | RECV_ONLY | NO_INSTALL, no injection

        STATE.with_borrow(|state| state.open_result as *mut c_void)
    }

    unsafe extern "C" fn recv(
        _handle: *mut c_void,
        output: *mut c_void,
        capacity: u32,
        length: *mut u32,
        address: *mut Address,
    ) -> c_int {
        STATE.with_borrow(|state| {
            assert!(state.bytes.len() <= capacity as usize);

            // SAFETY: production receive provides a writable SNAPLEN-byte buffer;
            // the synthetic packet length above is checked before copying.
            unsafe {
                ptr::copy_nonoverlapping(
                    state.bytes.as_ptr(),
                    output.cast::<u8>(),
                    state.bytes.len(),
                )
            };

            // SAFETY: production receive supplies an initialized writable u32.
            unsafe { *length = state.length.unwrap_or(state.bytes.len() as u32) };
            // SAFETY: production receive supplies an aligned 80-byte address.
            unsafe { (*address).flags = state.flags };
            state.recv_result
        })
    }

    unsafe extern "C" fn shutdown(_handle: *mut c_void, how: c_int) -> c_int {
        assert_eq!(how, SHUTDOWN_RECV);
        STATE.with_borrow_mut(|state| state.shutdown += 1);
        1
    }

    unsafe extern "C" fn close(_handle: *mut c_void) -> c_int {
        STATE.with_borrow_mut(|state| state.closed += 1);
        1
    }

    fn mock() -> WinDivert {
        STATE.with_borrow_mut(|state| *state = State::default());

        WinDivert { api: Api { open, recv, shutdown, close, _library: None } }
    }

    #[test]
    fn mock_capture_uses_passive_flags_and_closes_once() {
        let backend = mock();
        let capture = backend.open().expect("mock open");

        assert_eq!(capture.receive().expect("mock recv"), Receive::Packet(packet::tests::ipv4()));

        capture.shutdown().expect("mock shutdown");
        drop(capture);

        STATE.with_borrow(|state| {
            assert_eq!(state.shutdown, 1);
            assert_eq!(state.closed, 1);
        });
    }

    #[test]
    fn receive_rejects_outbound_or_ambiguous_metadata() {
        let backend = mock();
        let capture = backend.open().expect("mock open");

        for flags in [0, 1 << 17, (1 << 16) | (1 << 17), (1 << 16) | 1, (1 << 16) | (1 << 8)] {
            STATE.with_borrow_mut(|state| state.flags = flags);
            assert_eq!(capture.receive().expect("mock recv"), Receive::Discarded);
        }
    }

    #[test]
    fn receive_admits_both_server_ports_and_rejects_both_client_directions() {
        let backend = mock();
        let capture = backend.open().expect("mock open");

        for server in [3101_u16, 5222] {
            STATE.with_borrow_mut(|state| {
                state.bytes[20..22].copy_from_slice(&server.to_be_bytes());
                state.bytes[22..24].copy_from_slice(&45000_u16.to_be_bytes());
            });
            let expected = STATE.with_borrow(|state| state.bytes.clone());
            assert_eq!(capture.receive().expect("server packet"), Receive::Packet(expected));

            STATE.with_borrow_mut(|state| {
                state.bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
                state.bytes[22..24].copy_from_slice(&server.to_be_bytes());
            });
            assert_eq!(capture.receive().expect("client packet"), Receive::Discarded);

            for destination in [3101_u16, 5222] {
                STATE.with_borrow_mut(|state| {
                    state.bytes[20..22].copy_from_slice(&server.to_be_bytes());
                    state.bytes[22..24].copy_from_slice(&destination.to_be_bytes());
                });
                assert_eq!(capture.receive().expect("ambiguous pair"), Receive::Discarded);
            }
        }
    }

    #[test]
    fn invalid_lengths_or_client_packets_never_escape() {
        let backend = mock();
        let capture = backend.open().expect("mock open");

        STATE.with_borrow_mut(|state| state.length = Some(SNAPLEN as u32 + 1));
        assert!(matches!(capture.receive(), Err(Error::InvalidPacket(_))));

        STATE.with_borrow_mut(|state| {
            state.length = Some(0);
        });
        assert_eq!(capture.receive().expect("zero recv"), Receive::Discarded);

        STATE.with_borrow_mut(|state| {
            state.length = None;
            state.bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
        });
        assert_eq!(capture.receive().expect("client recv"), Receive::Discarded);
    }

    #[test]
    fn null_and_invalid_handles_are_never_closed() {
        for handle in [0, -1] {
            let backend = mock();
            STATE.with_borrow_mut(|state| state.open_result = handle);
            assert!(backend.open().is_err());
            STATE.with_borrow(|state| assert_eq!(state.closed, 0));
        }
    }

    #[test]
    fn errors_remain_actionable_without_requesting_elevation() {
        assert!(matches!(
            windows_error("open", std::io::Error::from_raw_os_error(5)),
            Error::PermissionDenied { .. }
        ));

        for code in [2, 577, 654, 1060, 1275, 1753] {
            assert!(
                matches!(windows_error("open", std::io::Error::from_raw_os_error(code)), Error::DriverUnavailable { code: value } if value == code)
            );
        }

        assert!(matches!(
            windows_error("open", std::io::Error::from_raw_os_error(87)),
            Error::Native { .. }
        ));
    }

    #[test]
    fn address_layout_matches_win64_abi() {
        assert_eq!(std::mem::size_of::<Address>(), 80);
        assert_eq!(std::mem::align_of::<Address>(), 8);
        assert_eq!(std::mem::offset_of!(Address, flags), 8);
        assert_eq!(std::mem::offset_of!(Address, data), 16);

        fn send_sync<T: Send + Sync>() {}
        send_sync::<Capture<'_>>();
    }
}
