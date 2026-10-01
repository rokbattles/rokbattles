use std::{
    ffi::{CStr, CString, c_char, c_int, c_uchar, c_uint, c_void},
    marker::PhantomData,
    path::Path,
    ptr::{self, NonNull},
    rc::Rc,
};

use libloading::Library;

use crate::{Error, Receive, SNAPLEN, library, packet};

const FILTER: &CStr = c"tcp src port 3101 and not dst port 3101";
const PCAP_D_IN: c_int = 1;
const ERRBUF_SIZE: usize = 256;

// ABI: libpcap pcap/pcap.h. timeval must come from the platform libc; c_long
// guesses would be wrong for some Unix ABIs. The pcap_t itself stays opaque.
#[repr(C)]
struct PacketHeader {
    timestamp: libc::timeval,
    captured_length: u32,
    original_length: u32,
}

#[repr(C)]
struct BpfProgram {
    length: c_uint,
    instructions: *mut c_void,
}

type Create = unsafe extern "C" fn(*const c_char, *mut c_char) -> *mut c_void;
type SetInt = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
type HandleInt = unsafe extern "C" fn(*mut c_void) -> c_int;
type SetNonblock = unsafe extern "C" fn(*mut c_void, c_int, *mut c_char) -> c_int;
type Compile =
    unsafe extern "C" fn(*mut c_void, *mut BpfProgram, *const c_char, c_int, u32) -> c_int;
type SetFilter = unsafe extern "C" fn(*mut c_void, *mut BpfProgram) -> c_int;
type FreeCode = unsafe extern "C" fn(*mut BpfProgram);
type Next =
    unsafe extern "C" fn(*mut c_void, *mut *const PacketHeader, *mut *const c_uchar) -> c_int;
type GetError = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type Close = unsafe extern "C" fn(*mut c_void);

struct Api {
    create: Create,
    snaplen: SetInt,
    promiscuous: SetInt,
    timeout: SetInt,
    activate: HandleInt,
    direction: SetInt,
    nonblock: SetNonblock,
    datalink: HandleInt,
    compile: Compile,
    setfilter: SetFilter,
    freecode: FreeCode,
    next: Next,
    geterr: GetError,
    close: Close,
    // Dropped only after every Capture borrow has ended; function pointers never
    // escape this owner. Tests use mocked functions and no library.
    _library: Option<Library>,
}

/// Loaded functions, without an active capture. Loading never opens an interface.
pub struct Pcap {
    api: Api,
}

impl Pcap {
    /// Load an explicitly selected libpcap implementation, without starting capture.
    ///
    /// # Safety
    /// The path and its dependencies must be trusted native code with the libpcap
    /// ABI (1.x). Loading runs library initializers. Use an administrator-controlled
    /// installation path and sanitized loader environment when elevated; an
    /// absolute top-level path cannot constrain Unix dependency resolution.
    pub unsafe fn load(path: &Path) -> Result<Self, Error> {
        // SAFETY: caller guarantees trust, dependencies and libpcap ABI.
        let library = unsafe { library::load(path) }?;
        macro_rules! symbol {
            ($name:expr) => {{
                // SAFETY: exact documented C signature, inferred from the field;
                // the resulting private pointer is paired with its live library.
                unsafe { library::symbol(&library, $name) }?
            }};
        }
        let api = Api {
            create: symbol!(c"pcap_create"),
            snaplen: symbol!(c"pcap_set_snaplen"),
            promiscuous: symbol!(c"pcap_set_promisc"),
            timeout: symbol!(c"pcap_set_timeout"),
            activate: symbol!(c"pcap_activate"),
            direction: symbol!(c"pcap_setdirection"),
            nonblock: symbol!(c"pcap_setnonblock"),
            datalink: symbol!(c"pcap_datalink"),
            compile: symbol!(c"pcap_compile"),
            setfilter: symbol!(c"pcap_setfilter"),
            freecode: symbol!(c"pcap_freecode"),
            next: symbol!(c"pcap_next_ex"),
            geterr: symbol!(c"pcap_geterr"),
            close: symbol!(c"pcap_close"),
            _library: Some(library),
        };
        Ok(Self { api })
    }

    /// Explicitly begin passive, non-promiscuous, inbound capture on one interface.
    /// Errors close the handle. No permission or network setting is changed.
    /// Remote rpcap URLs are rejected; the caller must select a local interface.
    pub fn open(&self, interface: &str) -> Result<Capture<'_>, Error> {
        if interface.is_empty() || interface.len() > 255 || interface.contains("://") {
            return Err(Error::InvalidInput("expected a local interface name"));
        }
        let interface = CString::new(interface)
            .map_err(|_error| Error::InvalidInput("interface contains NUL"))?;
        let mut error_buffer = [0; ERRBUF_SIZE];
        // SAFETY: both C string and writable fixed-size errbuf remain live.
        let handle = unsafe { (self.api.create)(interface.as_ptr(), error_buffer.as_mut_ptr()) };
        let handle = NonNull::new(handle).ok_or_else(|| Error::Native {
            operation: "pcap_create",
            detail: buffer_message(&error_buffer),
        })?;
        let mut capture =
            Capture { api: &self.api, handle, link_type: 0, _single_thread: PhantomData };
        for (set, value, operation) in [
            (self.api.snaplen, SNAPLEN as c_int, "pcap_set_snaplen"),
            (self.api.promiscuous, 0, "pcap_set_promisc"),
            (self.api.timeout, 100, "pcap_set_timeout"),
        ] {
            // SAFETY: handle is newly created, open and exclusively used here.
            let status = unsafe { set(handle.as_ptr(), value) };
            capture.check(status, operation)?;
        }
        // SAFETY: configured, not-yet-activated live handle owned by capture.
        let status = unsafe { (self.api.activate)(handle.as_ptr()) };
        // Positive results are warnings, not failures, but rejecting them is
        // deliberate: never quietly relax the requested capture configuration.
        capture.check(status, "pcap_activate")?;
        // SAFETY: activated handle; PCAP_D_IN is the documented inbound enum.
        let status = unsafe { (self.api.direction)(handle.as_ptr(), PCAP_D_IN) };
        capture.check(status, "pcap_setdirection (inbound required)")?;
        // SAFETY: live handle and writable errbuf; makes receive nonblocking.
        let status = unsafe { (self.api.nonblock)(handle.as_ptr(), 1, error_buffer.as_mut_ptr()) };
        capture.check(status, "pcap_setnonblock")?;
        // SAFETY: live activated handle.
        let link_type = unsafe { (self.api.datalink)(handle.as_ptr()) };
        if !supported_link_type(link_type) {
            return Err(Error::UnsupportedLinkType(link_type));
        }
        capture.link_type = link_type;
        let mut program = BpfProgram { length: 0, instructions: ptr::null_mut() };
        // SAFETY: initialized output, valid constant filter string, owned handle.
        let status = unsafe {
            (self.api.compile)(handle.as_ptr(), &raw mut program, FILTER.as_ptr(), 1, u32::MAX)
        };
        capture.check(status, "pcap_compile")?;
        // SAFETY: compile succeeded; this exact program is live until freecode.
        let status = unsafe { (self.api.setfilter)(handle.as_ptr(), &raw mut program) };
        // SAFETY: a successfully compiled program must be freed exactly once,
        // including when setfilter fails. libpcap retains its own filter copy.
        unsafe { (self.api.freecode)(&raw mut program) };
        capture.check(status, "pcap_setfilter")?;
        Ok(capture)
    }
}

/// A single-thread-confined capture which borrows its loaded library owner.
pub struct Capture<'a> {
    api: &'a Api,
    handle: NonNull<c_void>,
    link_type: c_int,
    _single_thread: PhantomData<Rc<()>>,
}

impl Capture<'_> {
    fn check(&self, status: c_int, operation: &'static str) -> Result<(), Error> {
        if status == 0 {
            return Ok(());
        }
        let detail = self.error_message();
        if status == -8 || status == -11 {
            Err(Error::PermissionDenied { backend: "pcap", detail })
        } else {
            Err(Error::Native { operation, detail: format!("{detail} (status {status})") })
        }
    }

    fn error_message(&self) -> String {
        // SAFETY: handle remains open and its library is retained by the borrow.
        let pointer = unsafe { (self.api.geterr)(self.handle.as_ptr()) };
        if pointer.is_null() {
            return "native library supplied no diagnostic".to_owned();
        }
        // SAFETY: libpcap guarantees a NUL-terminated error string valid until
        // the next handle operation. Copy before calling the library again.
        unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned()
    }

    /// Read at most one native result. Rejected bytes never leave this adapter.
    pub fn receive(&mut self) -> Result<Receive, Error> {
        let mut header = ptr::null();
        let mut data = ptr::null();
        // SAFETY: valid exclusive handle, initialized writable pointer outputs.
        let status =
            unsafe { (self.api.next)(self.handle.as_ptr(), &raw mut header, &raw mut data) };
        match status {
            0 => Ok(Receive::Idle),
            -2 => Ok(Receive::End),
            1 => {
                if header.is_null() || data.is_null() {
                    return Err(Error::InvalidPacket("pcap returned NULL packet pointers"));
                }
                // SAFETY: next_ex succeeded; libpcap owns a valid aligned header
                // until the next receive call, excluded by this mutable borrow.
                let header = unsafe { &*header };
                let length = usize::try_from(header.captured_length)
                    .map_err(|_error| Error::InvalidPacket("capture length overflow"))?;
                if length == 0
                    || length > SNAPLEN
                    || header.captured_length != header.original_length
                {
                    return Err(Error::InvalidPacket("truncated or oversized pcap packet"));
                }
                // SAFETY: trusted libpcap guarantees `captured_length` bytes.
                // The length was bounded before forming this borrowed slice.
                let bytes = unsafe { std::slice::from_raw_parts(data, length) };
                Ok(link_packet(self.link_type, bytes)
                    .and_then(packet::server_packet)
                    .map_or(Receive::Discarded, |ip| Receive::Packet(ip.to_vec())))
            }
            _ => Err(Error::Native { operation: "pcap_next_ex", detail: self.error_message() }),
        }
    }
}

impl Drop for Capture<'_> {
    fn drop(&mut self) {
        // SAFETY: unique owner closes exactly once, while Api and Library live.
        unsafe { (self.api.close)(self.handle.as_ptr()) };
    }
}

fn buffer_message(buffer: &[c_char; ERRBUF_SIZE]) -> String {
    let bytes: Vec<_> =
        buffer.iter().take_while(|byte| **byte != 0).map(|byte| byte.cast_unsigned()).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn supported_link_type(kind: c_int) -> bool {
    matches!(kind, 0 | 1 | 12 | 108 | 113 | 228 | 229 | 276)
}

fn link_packet(kind: c_int, bytes: &[u8]) -> Option<&[u8]> {
    match kind {
        12 => Some(bytes), // DLT_RAW (Linux/macOS)
        228 if bytes.first()? >> 4 == 4 => Some(bytes),
        229 if bytes.first()? >> 4 == 6 => Some(bytes),
        1 => match bytes.get(12..14)? {
            // Ethernet, without VLAN guessing.
            [0x08, 0x00] | [0x86, 0xdd] => bytes.get(14..),
            _ => None,
        },
        0 | 108 => {
            let family: [u8; 4] = bytes.get(..4)?.try_into().ok()?;
            let family =
                if kind == 0 { u32::from_ne_bytes(family) } else { u32::from_be_bytes(family) };
            if family == libc::AF_INET as u32 || family == libc::AF_INET6 as u32 {
                bytes.get(4..)
            } else {
                None
            }
        }
        113 => {
            // Linux cooked capture: reject outgoing and non-unicast data too.
            if bytes.get(..2)? != [0, 0] {
                return None;
            }
            match bytes.get(14..16)? {
                [0x08, 0x00] | [0x86, 0xdd] => bytes.get(16..),
                _ => None,
            }
        }
        276 => {
            if bytes.get(10)? != &0 {
                return None;
            }
            match bytes.get(..2)? {
                [0x08, 0x00] | [0x86, 0xdd] => bytes.get(20..),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    struct State {
        fail: &'static str,
        status: c_int,
        calls: Vec<&'static str>,
        closed: usize,
        freed: usize,
        header: PacketHeader,
        bytes: Vec<u8>,
        next_status: c_int,
        null_header: bool,
        null_data: bool,
        link_type: c_int,
    }

    impl Default for State {
        fn default() -> Self {
            let bytes = packet::tests::ipv4();
            Self {
                fail: "",
                status: -1,
                calls: Vec::new(),
                closed: 0,
                freed: 0,
                header: PacketHeader {
                    timestamp: libc::timeval { tv_sec: 0, tv_usec: 0 },
                    captured_length: bytes.len() as u32,
                    original_length: bytes.len() as u32,
                },
                bytes,
                next_status: 1,
                null_header: false,
                null_data: false,
                link_type: 12,
            }
        }
    }

    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

    fn call(name: &'static str) -> c_int {
        STATE.with_borrow_mut(|state| {
            state.calls.push(name);
            if state.fail == name { state.status } else { 0 }
        })
    }

    unsafe extern "C" fn create(_interface: *const c_char, _error: *mut c_char) -> *mut c_void {
        if call("create") == 0 { ptr::dangling_mut::<u8>().cast() } else { ptr::null_mut() }
    }
    unsafe extern "C" fn snaplen(_handle: *mut c_void, value: c_int) -> c_int {
        assert_eq!(value, SNAPLEN as c_int);
        call("snaplen")
    }
    unsafe extern "C" fn promiscuous(_handle: *mut c_void, value: c_int) -> c_int {
        assert_eq!(value, 0);
        call("promiscuous")
    }
    unsafe extern "C" fn timeout(_handle: *mut c_void, value: c_int) -> c_int {
        assert_eq!(value, 100);
        call("timeout")
    }
    unsafe extern "C" fn activate(_handle: *mut c_void) -> c_int {
        call("activate")
    }
    unsafe extern "C" fn direction(_handle: *mut c_void, value: c_int) -> c_int {
        assert_eq!(value, PCAP_D_IN);
        call("direction")
    }
    unsafe extern "C" fn nonblock(
        _handle: *mut c_void,
        value: c_int,
        _error: *mut c_char,
    ) -> c_int {
        assert_eq!(value, 1);
        call("nonblock")
    }
    unsafe extern "C" fn datalink(_handle: *mut c_void) -> c_int {
        STATE.with_borrow(|state| state.link_type)
    }
    unsafe extern "C" fn compile(
        _handle: *mut c_void,
        _program: *mut BpfProgram,
        filter: *const c_char,
        optimize: c_int,
        mask: u32,
    ) -> c_int {
        // SAFETY: production open supplies a valid static filter C string.
        assert_eq!(unsafe { CStr::from_ptr(filter) }, FILTER);
        assert_eq!(optimize, 1);
        assert_eq!(mask, u32::MAX);
        call("compile")
    }
    unsafe extern "C" fn setfilter(_handle: *mut c_void, _program: *mut BpfProgram) -> c_int {
        call("setfilter")
    }
    unsafe extern "C" fn freecode(_program: *mut BpfProgram) {
        STATE.with_borrow_mut(|state| state.freed += 1);
    }
    unsafe extern "C" fn close(_handle: *mut c_void) {
        STATE.with_borrow_mut(|state| state.closed += 1);
    }
    unsafe extern "C" fn geterr(_handle: *mut c_void) -> *const c_char {
        c"mock capture error".as_ptr()
    }
    unsafe extern "C" fn next(
        _handle: *mut c_void,
        header: *mut *const PacketHeader,
        data: *mut *const u8,
    ) -> c_int {
        STATE.with_borrow(|state| {
            // SAFETY: production receive passes valid pointer outputs. State
            // owns these backing allocations until the next test mutation.
            unsafe {
                *header = if state.null_header { ptr::null() } else { &raw const state.header }
            };
            // SAFETY: as above; separate output assigned its retained allocation.
            unsafe { *data = if state.null_data { ptr::null() } else { state.bytes.as_ptr() } };
            state.next_status
        })
    }

    fn mock() -> Pcap {
        STATE.with_borrow_mut(|state| *state = State::default());
        Pcap {
            api: Api {
                create,
                snaplen,
                promiscuous,
                timeout,
                activate,
                direction,
                nonblock,
                datalink,
                compile,
                setfilter,
                freecode,
                next,
                geterr,
                close,
                _library: None,
            },
        }
    }

    #[test]
    fn open_installs_strict_filter_and_drop_closes_once() {
        let api = mock();
        let mut capture = api.open("test0").expect("mock open");
        assert_eq!(
            capture.receive().expect("mock receive"),
            Receive::Packet(packet::tests::ipv4())
        );
        STATE.with_borrow(|state| {
            assert_eq!(
                state.calls,
                [
                    "create",
                    "snaplen",
                    "promiscuous",
                    "timeout",
                    "activate",
                    "direction",
                    "nonblock",
                    "compile",
                    "setfilter"
                ]
            );
            assert_eq!(state.freed, 1);
            assert_eq!(state.closed, 0);
        });
        drop(capture);
        STATE.with_borrow(|state| assert_eq!(state.closed, 1));
    }

    #[test]
    fn every_failed_setup_step_closes_and_never_exposes_capture() {
        for step in [
            "snaplen",
            "promiscuous",
            "timeout",
            "activate",
            "direction",
            "nonblock",
            "compile",
            "setfilter",
        ] {
            let api = mock();
            STATE.with_borrow_mut(|state| state.fail = step);
            assert!(api.open("test0").is_err(), "{step}");
            STATE.with_borrow(|state| {
                assert_eq!(state.closed, 1, "{step}");
                assert_eq!(state.freed, usize::from(step == "setfilter"), "{step}");
            });
        }
    }

    #[test]
    fn permissions_and_activation_warnings_fail_closed() {
        for status in [-8, -11, 1, 2] {
            let api = mock();
            STATE.with_borrow_mut(|state| {
                state.fail = "activate";
                state.status = status;
            });
            let error = match api.open("test0") {
                Ok(_) => panic!("must fail"),
                Err(error) => error,
            };
            assert_eq!(matches!(error, Error::PermissionDenied { .. }), status < 0);
            STATE.with_borrow(|state| assert_eq!(state.closed, 1));
        }
    }

    #[test]
    fn invalid_interface_never_invokes_create() {
        let api = mock();
        for interface in ["", "rpcap://remote/interface", "bad\0name"] {
            assert!(matches!(api.open(interface), Err(Error::InvalidInput(_))));
        }
        STATE.with_borrow(|state| assert!(state.calls.is_empty()));
    }

    #[test]
    fn null_handle_and_unsupported_link_type_are_errors() {
        let api = mock();
        STATE.with_borrow_mut(|state| state.fail = "create");
        assert!(matches!(api.open("test0"), Err(Error::Native { .. })));
        STATE.with_borrow(|state| assert_eq!(state.closed, 0));
        let api = mock();
        STATE.with_borrow_mut(|state| state.link_type = 999);
        assert!(matches!(api.open("test0"), Err(Error::UnsupportedLinkType(999))));
        STATE.with_borrow(|state| assert_eq!(state.closed, 1));
    }

    #[test]
    fn receive_handles_idle_end_error_and_rejects_client_packets() {
        let api = mock();
        let mut capture = api.open("test0").expect("mock open");
        for (status, expected) in [(0, Receive::Idle), (-2, Receive::End)] {
            STATE.with_borrow_mut(|state| state.next_status = status);
            assert_eq!(capture.receive().expect("non-packet"), expected);
        }
        STATE.with_borrow_mut(|state| state.next_status = -1);
        assert!(matches!(capture.receive(), Err(Error::Native { .. })));
        STATE.with_borrow_mut(|state| {
            state.next_status = 1;
            state.bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
        });
        assert_eq!(capture.receive().expect("client discarded"), Receive::Discarded);
    }

    #[test]
    fn receive_checks_lengths_and_null_outputs_before_dereferencing_data() {
        let api = mock();
        let mut capture = api.open("test0").expect("mock open");
        for mode in 0..5 {
            STATE.with_borrow_mut(|state| {
                state.null_header = mode == 0;
                state.null_data = mode == 1;
                state.header.captured_length = match mode {
                    2 => 0,
                    3 => SNAPLEN as u32 + 1,
                    _ => 41,
                };
                state.header.original_length =
                    if mode == 4 { 42 } else { state.header.captured_length };
            });
            assert!(matches!(capture.receive(), Err(Error::InvalidPacket(_))));
        }
    }

    #[test]
    fn supported_link_headers_are_bounded_and_cooked_outgoing_is_rejected() {
        let ip = packet::tests::ipv4();
        for (kind, mut header) in [
            (1, vec![0; 14]),
            (113, vec![0; 16]),
            (276, vec![0; 20]),
            (0, (libc::AF_INET as u32).to_ne_bytes().to_vec()),
            (108, (libc::AF_INET as u32).to_be_bytes().to_vec()),
        ] {
            let size = header.len();
            match kind {
                1 => header[12..14].copy_from_slice(&[8, 0]),
                113 => header[14..16].copy_from_slice(&[8, 0]),
                276 => header[..2].copy_from_slice(&[8, 0]),
                _ => {}
            }
            header.extend_from_slice(&ip);
            assert_eq!(link_packet(kind, &header), Some(ip.as_slice()));
            for length in 0..size {
                assert_eq!(link_packet(kind, &header[..length]), None);
            }
            if kind == 113 {
                header[1] = 4;
                assert_eq!(link_packet(kind, &header), None);
            }
            if kind == 276 {
                header[10] = 4;
                assert_eq!(link_packet(kind, &header), None);
            }
        }
    }
}
