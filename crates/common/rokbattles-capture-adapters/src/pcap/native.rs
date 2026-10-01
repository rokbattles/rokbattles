use std::{
    ffi::{CStr, CString, c_char, c_int, c_uchar, c_uint, c_void},
    marker::PhantomData,
    net::IpAddr,
    path::Path,
    ptr::{self, NonNull},
    rc::Rc,
};

use libloading::Library;

use crate::{Error, Receive, SNAPLEN, library, packet};

const PORT_FILTER: &str =
    "tcp and (src port 3101 or src port 5222) and not (dst port 3101 or dst port 5222)";
const MAX_CLIENT_ADDRESSES: usize = 16;

#[cfg(unix)]
const PCAP_D_IN: c_int = 1;
#[cfg(unix)]
const PCAP_D_OUT: c_int = 2;

const ERRBUF_SIZE: usize = 256;

// ABI: pcap/pcap.h uses the platform timeval. Windows Winsock uses two
// 32-bit C longs (LLP64), even on x64/ARM64. Unix uses its actual libc layout.
#[cfg(unix)]
type Timeval = libc::timeval;

#[cfg(windows)]
#[repr(C)]
struct Timeval {
    tv_sec: i32,
    tv_usec: i32,
}

#[cfg(unix)]
const LOOP_IPV4: u32 = libc::AF_INET as u32;
#[cfg(unix)]
const LOOP_IPV6: u32 = libc::AF_INET6 as u32;

#[cfg(windows)]
const LOOP_IPV4: u32 = 2;
#[cfg(windows)]
const LOOP_IPV6: u32 = 24; // Npcap DLT_NULL uses BSD values, not Winsock AF_INET6.

// Serialize crate-originated initialization; the DLL may unload/reload, so a
// process-lifetime Once would be incorrect. Upstream accepts same-mode repeats.
#[cfg(windows)]
static INIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(windows)]
type Init = unsafe extern "C" fn(c_uint, *mut c_char) -> c_int;

#[repr(C)]
struct PacketHeader {
    timestamp: Timeval,
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
    #[cfg(windows)]
    init: Init,
    create: Create,
    snaplen: SetInt,
    promiscuous: SetInt,
    timeout: SetInt,
    activate: HandleInt,
    #[cfg(unix)]
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
    /// Windows requires modern Npcap with pcap_init (libpcap >= 1.9); the caller
    /// must also exclude concurrent pcap_init calls from outside this crate.
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
            #[cfg(windows)]
            init: symbol!(c"pcap_init"),
            create: symbol!(c"pcap_create"),
            snaplen: symbol!(c"pcap_set_snaplen"),
            promiscuous: symbol!(c"pcap_set_promisc"),
            timeout: symbol!(c"pcap_set_timeout"),
            activate: symbol!(c"pcap_activate"),
            #[cfg(unix)]
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

    /// Whether this backend proves outbound packet direction independently of
    /// packet addresses. Npcap records do not, so Windows pcap stays server-only.
    pub const fn client_controls_supported() -> bool {
        cfg!(unix)
    }

    /// Open server capture and, on Unix, a separate outbound-control handle.
    /// Each handle has its own direction restriction and fixed narrow BPF.
    /// Failure to establish either required direction closes both handles.
    /// The caller supplies current local interface addresses; reopen on changes.
    pub fn open(&self, interface: &str, clients: &[IpAddr]) -> Result<Capture<'_>, Error> {
        let server = self.open_direction(interface, clients, false)?;
        #[cfg(unix)]
        let client = self.open_direction(interface, clients, true)?;
        Ok(Capture {
            server,
            #[cfg(unix)]
            client,
            #[cfg(unix)]
            pending_server: None,
            #[cfg(unix)]
            pending_client: None,
            #[cfg(unix)]
            last_emitted: (0, 0),
        })
    }

    fn open_direction(
        &self,
        interface: &str,
        clients: &[IpAddr],
        outbound: bool,
    ) -> Result<NativeCapture<'_>, Error> {
        if interface.is_empty() || interface.len() > 255 || interface.contains("://") {
            return Err(Error::InvalidInput("expected a local interface name"));
        }

        let interface = CString::new(interface)
            .map_err(|_error| Error::InvalidInput("interface contains NUL"))?;
        let (clients, filter) = client_filter(clients, outbound)?;
        let mut error_buffer = [0; ERRBUF_SIZE];

        #[cfg(windows)]
        {
            let _guard = INIT_LOCK.lock().map_err(|_error| Error::Native {
                operation: "pcap_init",
                detail: "initialization lock poisoned".to_owned(),
            })?;

            // SAFETY: retained trusted Npcap ABI, writable errbuf; UTF8 mode (1)
            // disables pcap_create's unsafe legacy UTF16 string probe. No handle
            // is opened until initialization succeeds. All our calls serialize.
            let status = unsafe { (self.api.init)(1, error_buffer.as_mut_ptr()) };
            if status != 0 {
                return Err(Error::Native {
                    operation: "pcap_init",
                    detail: buffer_message(&error_buffer),
                });
            }
        }

        // SAFETY: both C string and writable fixed-size errbuf remain live.
        let handle = unsafe { (self.api.create)(interface.as_ptr(), error_buffer.as_mut_ptr()) };
        let handle = NonNull::new(handle).ok_or_else(|| Error::Native {
            operation: "pcap_create",
            detail: buffer_message(&error_buffer),
        })?;
        let mut capture = NativeCapture {
            api: &self.api,
            handle,
            link_type: 0,
            clients,
            outbound,
            timestamp: (0, 0),
            _single_thread: PhantomData,
        };

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

        #[cfg(unix)]
        {
            let direction = if outbound { PCAP_D_OUT } else { PCAP_D_IN };
            // SAFETY: activated handle and documented direction enum. Refuse
            // unsupported directions rather than inferring them from addresses.
            let status = unsafe { (self.api.direction)(handle.as_ptr(), direction) };
            capture.check(status, "pcap_setdirection (exact direction required)")?;
        }

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
        // SAFETY: initialized output, live CString built only from fixed grammar
        // and typed IP addresses; owned handle. No arbitrary BPF is accepted.
        let status = unsafe {
            (self.api.compile)(handle.as_ptr(), &raw mut program, filter.as_ptr(), 1, u32::MAX)
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

/// Passive server packets plus, where proven, local zero-payload controls.
/// Unix merges one pending typed result per direction by native timestamp.
/// Client bytes never enter either pending slot. Equal timestamps prefer controls;
/// ambiguous handshake ordering fails closed in the lifecycle observer.
pub struct Capture<'a> {
    server: NativeCapture<'a>,
    #[cfg(unix)]
    client: NativeCapture<'a>,
    #[cfg(unix)]
    pending_server: Option<(Receive, (i128, i128))>,
    #[cfg(unix)]
    pending_client: Option<(Receive, (i128, i128))>,
    #[cfg(unix)]
    last_emitted: (i128, i128),
}

impl Capture<'_> {
    /// Poll at most one native packet per direction, without blocking.
    pub fn receive(&mut self) -> Result<Receive, Error> {
        #[cfg(windows)]
        {
            self.server.receive()
        }
        #[cfg(unix)]
        {
            let mut discarded = false;
            for (capture, pending) in [
                (&mut self.client, &mut self.pending_client),
                (&mut self.server, &mut self.pending_server),
            ] {
                if pending.is_none() {
                    match capture.receive()? {
                        packet @ (Receive::Packet(_) | Receive::ClientControl(_)) => {
                            *pending = Some((packet, capture.timestamp));
                        }
                        Receive::End => return Ok(Receive::End),
                        Receive::Discarded => discarded = true,
                        Receive::Idle => {}
                    }
                }
            }
            let take_client = match (&self.pending_client, &self.pending_server) {
                (Some((_, client)), Some((_, server))) => client <= server,
                (Some(_), None) => true,
                _ => false,
            };
            let pending =
                if take_client { &mut self.pending_client } else { &mut self.pending_server };
            let Some((mut packet, timestamp)) = pending.take() else {
                return Ok(if discarded { Receive::Discarded } else { Receive::Idle });
            };
            if timestamp < self.last_emitted {
                if let Receive::Packet(bytes) = &mut packet {
                    zeroize::Zeroize::zeroize(bytes);
                }
                return Err(Error::InvalidPacket("capture direction ordering regressed"));
            }
            self.last_emitted = timestamp;
            Ok(packet)
        }
    }
}

#[cfg(unix)]
impl Drop for Capture<'_> {
    fn drop(&mut self) {
        for pending in [&mut self.pending_server, &mut self.pending_client] {
            if let Some((Receive::Packet(bytes), _)) = pending {
                zeroize::Zeroize::zeroize(bytes);
            }
        }
    }
}

/// One single-thread-confined native direction and its independently fixed filter.
struct NativeCapture<'a> {
    api: &'a Api,
    handle: NonNull<c_void>,
    link_type: c_int,
    clients: Vec<IpAddr>,
    outbound: bool,
    timestamp: (i128, i128),
    _single_thread: PhantomData<Rc<()>>,
}

impl NativeCapture<'_> {
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
                // Establish each native output's non-null invariant separately
                // before any dereference. Native allocation/provenance validity
                // still relies on the trusted libpcap ABI, not on NonNull alone.
                let header = NonNull::new(header.cast_mut())
                    .ok_or(Error::InvalidPacket("pcap returned NULL header pointer"))?;
                let data = NonNull::new(data.cast_mut())
                    .ok_or(Error::InvalidPacket("pcap returned NULL data pointer"))?;

                // SAFETY: next_ex succeeded; libpcap owns a valid aligned header
                // until the next receive call, excluded by this mutable borrow.
                let header = unsafe { header.as_ref() };
                let seconds = i128::from(header.timestamp.tv_sec);
                let micros = i128::from(header.timestamp.tv_usec);
                if seconds < 0 || !(0..1_000_000).contains(&micros) {
                    return Err(Error::InvalidPacket("invalid native observation timestamp"));
                }
                if (seconds, micros) < self.timestamp {
                    return Err(Error::InvalidPacket("native observation timestamp regressed"));
                }
                self.timestamp = (seconds, micros);
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
                let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr(), length) };
                let Some(ip) = link_packet(self.link_type, bytes) else {
                    return Ok(Receive::Discarded);
                };
                // Unix handle direction proves IN/OUT even for Ethernet/raw.
                // Cooked link metadata must agree too. Windows never admits
                // controls because Npcap does not provide an equivalent proof.
                let direction = cooked_direction(self.link_type, bytes);
                if !self.outbound
                    && direction != Some(true)
                    && let Some(ip) = packet::server_packet(ip)
                    && packet::destination(ip)
                        .is_some_and(|address| self.clients.contains(&address))
                {
                    return Ok(Receive::Packet(ip.to_vec()));
                }
                #[cfg(unix)]
                if self.outbound
                    && direction != Some(false)
                    && let Some(control) = packet::client_control(ip)
                    && self.clients.contains(&control.key.client.ip())
                {
                    return Ok(Receive::ClientControl(control));
                }
                Ok(Receive::Discarded)
            }
            _ => Err(Error::Native { operation: "pcap_next_ex", detail: self.error_message() }),
        }
    }
}

impl Drop for NativeCapture<'_> {
    fn drop(&mut self) {
        // SAFETY: unique owner closes exactly once, while Api and Library live.
        unsafe { (self.api.close)(self.handle.as_ptr()) };
    }
}

fn client_filter(clients: &[IpAddr], outbound: bool) -> Result<(Vec<IpAddr>, CString), Error> {
    if clients.is_empty() || clients.len() > MAX_CLIENT_ADDRESSES {
        return Err(Error::InvalidInput("expected 1..=16 local client addresses"));
    }

    let mut unique = Vec::with_capacity(clients.len());
    for address in clients {
        if address.is_unspecified()
            || address.is_multicast()
            || matches!(address, IpAddr::V4(ip) if ip.is_broadcast())
        {
            return Err(Error::InvalidInput("client addresses must be specific unicast IPs"));
        }
        if !unique.contains(address) {
            unique.push(*address);
        }
    }

    let destinations =
        unique.iter().map(|address| format!("dst host {address}")).collect::<Vec<_>>().join(" or ");
    if !outbound {
        let filter = CString::new(format!("{PORT_FILTER} and ({destinations})"))
            .map_err(|_error| Error::InvalidInput("invalid address filter"))?;
        return Ok((unique, filter));
    }
    let sources =
        unique.iter().map(|address| format!("src host {address}")).collect::<Vec<_>>().join(" or ");
    // IPv6 TCP byte access is explicitly IP-relative: libpcap tcp[] arithmetic
    // only handles IPv4. Both forms reject fragments/extensions and require the
    // full IP TCP length to equal the advertised TCP header length.
    let flags = |field: &str| {
        [1, 2, 4, 16, 17, 20]
            .iter()
            .map(|flag| format!("({field} & 63) = {flag}"))
            .collect::<Vec<_>>()
            .join(" or ")
    };
    let v4 = format!(
        "(ip and (ip[0] & 15) >= 5 and ip[9] = 6 and (ip[6:2] & 16383) = 0 and \
         (tcp[12] & 240) >= 80 and \
         ip[2:2] = ((ip[0] & 15) * 4 + (tcp[12] & 240) / 4) and ({}))",
        flags("tcp[13]")
    );
    let v6 = format!(
        "(ip6 and ip6[6] = 6 and (ip6[52] & 240) >= 80 and \
         ip6[4:2] = (ip6[52] & 240) / 4 and ({}))",
        flags("ip6[53]")
    );
    let filter = CString::new(format!(
        "(tcp and (dst port 3101 or dst port 5222) and \
         not (src port 0 or src port 3101 or src port 5222) and \
         ({sources}) and ({v4} or {v6}))"
    ))
    .map_err(|_error| Error::InvalidInput("invalid address filter"))?;

    Ok((unique, filter))
}

fn buffer_message(buffer: &[c_char; ERRBUF_SIZE]) -> String {
    let bytes: Vec<_> = buffer
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| u8::from_ne_bytes(byte.to_ne_bytes()))
        .collect();

    String::from_utf8_lossy(&bytes).into_owned()
}

fn supported_link_type(kind: c_int) -> bool {
    matches!(kind, 0 | 1 | 12 | 108 | 113 | 228 | 229 | 276)
}

fn cooked_direction(kind: c_int, bytes: &[u8]) -> Option<bool> {
    match kind {
        113 => Some(*bytes.get(1)? == 4),
        276 => Some(*bytes.get(10)? == 4),
        _ => None,
    }
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
            if family == LOOP_IPV4 || family == LOOP_IPV6 { bytes.get(4..) } else { None }
        }
        113 => {
            // Only unicast incoming (0) or outgoing (4); direction is checked
            // against the parsed server/client variant before admission.
            if !matches!(bytes.get(..2)?, [0, 0] | [0, 4]) {
                return None;
            }

            match bytes.get(14..16)? {
                [0x08, 0x00] | [0x86, 0xdd] => bytes.get(16..),
                _ => None,
            }
        }
        276 => {
            if !matches!(bytes.get(10)?, 0 | 4) {
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

    const CLIENT: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 2));

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
        #[cfg(unix)]
        directions: Vec<c_int>,
        #[cfg(unix)]
        fail_outbound: bool,
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
                    timestamp: Timeval { tv_sec: 0, tv_usec: 0 },
                    captured_length: bytes.len() as u32,
                    original_length: bytes.len() as u32,
                },
                bytes,
                next_status: 1,
                null_header: false,
                null_data: false,
                link_type: 12,
                #[cfg(unix)]
                directions: Vec::new(),
                #[cfg(unix)]
                fail_outbound: false,
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

    #[cfg(windows)]
    unsafe extern "C" fn init(encoding: c_uint, error: *mut c_char) -> c_int {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static ACTIVE: AtomicUsize = AtomicUsize::new(0);

        assert_eq!(encoding, 1);
        assert_eq!(ACTIVE.fetch_add(1, Ordering::SeqCst), 0, "initialization overlapped");
        std::thread::yield_now();

        // SAFETY: production open provides a zeroed, writable 256-byte errbuf.
        assert_eq!(unsafe { *error }, 0);
        let result = call("init");
        if result != 0 {
            let message = c"mock UTF8 initialization failure";
            // SAFETY: message including NUL is smaller than the supplied errbuf.
            unsafe {
                ptr::copy_nonoverlapping(message.as_ptr(), error, message.to_bytes_with_nul().len())
            };
        }

        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        result
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

    #[cfg(unix)]
    unsafe extern "C" fn direction(_handle: *mut c_void, value: c_int) -> c_int {
        assert!(matches!(value, PCAP_D_IN | PCAP_D_OUT));
        let fail = STATE.with_borrow_mut(|state| {
            state.directions.push(value);
            state.fail_outbound && value == PCAP_D_OUT
        });
        if fail { -1 } else { call("direction") }
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
        // SAFETY: production open supplies a live NUL-terminated filter CString.
        let filter = unsafe { CStr::from_ptr(filter) };
        assert!(
            filter == client_filter(&[CLIENT], false).expect("server filter").1.as_c_str()
                || filter == client_filter(&[CLIENT], true).expect("control filter").1.as_c_str()
        );
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
                #[cfg(windows)]
                init,
                create,
                snaplen,
                promiscuous,
                timeout,
                activate,
                #[cfg(unix)]
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
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");

        assert_eq!(
            capture.server.receive().expect("mock receive"),
            Receive::Packet(packet::tests::ipv4())
        );

        STATE.with_borrow(|state| {
            assert_eq!(
                state.calls,
                [
                    #[cfg(windows)]
                    "init",
                    "create",
                    "snaplen",
                    "promiscuous",
                    "timeout",
                    "activate",
                    #[cfg(unix)]
                    "direction",
                    "nonblock",
                    "compile",
                    "setfilter"
                ]
                .repeat(if cfg!(unix) { 2 } else { 1 })
            );
            #[cfg(unix)]
            assert_eq!(state.directions, [PCAP_D_IN, PCAP_D_OUT]);
            assert_eq!(state.freed, if cfg!(unix) { 2 } else { 1 });
            assert_eq!(state.closed, 0);
        });

        drop(capture);
        STATE.with_borrow(|state| assert_eq!(state.closed, if cfg!(unix) { 2 } else { 1 }));
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_outbound_direction_closes_both_handles() {
        let api = mock();
        STATE.with_borrow_mut(|state| state.fail_outbound = true);
        assert!(api.open("test0", &[CLIENT]).is_err());
        STATE.with_borrow(|state| {
            assert_eq!(state.directions, [PCAP_D_IN, PCAP_D_OUT]);
            assert_eq!(state.closed, 2);
        });
    }

    #[test]
    fn inbound_handle_never_admits_spoofed_local_source_controls() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
        STATE.with_borrow_mut(|state| {
            state.bytes = packet::tests::client_ipv4(0x04);
            state.header.captured_length = 40;
            state.header.original_length = 40;
        });
        assert_eq!(capture.server.receive().expect("spoofed inbound control"), Receive::Discarded);
        assert_eq!(Pcap::client_controls_supported(), cfg!(unix));
    }

    #[cfg(unix)]
    #[test]
    fn paired_receive_merges_timestamps_client_first_for_ties_and_rejects_late_records() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
        STATE.with_borrow_mut(|state| state.next_status = 0);
        let control = packet::client_control(&packet::tests::client_ipv4(0x02)).expect("control");
        capture.pending_client = Some((Receive::ClientControl(control), (1, 1)));
        capture.pending_server = Some((Receive::Packet(packet::tests::ipv4()), (1, 2)));
        assert_eq!(capture.receive().expect("earlier client"), Receive::ClientControl(control));
        assert!(matches!(capture.receive(), Ok(Receive::Packet(_))));
        capture.pending_client = Some((Receive::ClientControl(control), (2, 1)));
        capture.pending_server = Some((Receive::Packet(packet::tests::ipv4()), (2, 1)));
        assert_eq!(capture.receive().expect("tie client"), Receive::ClientControl(control));
        assert!(matches!(capture.receive(), Ok(Receive::Packet(_))));
        capture.pending_client = Some((Receive::ClientControl(control), (1, 3)));
        assert!(matches!(capture.receive(), Err(Error::InvalidPacket(_))));
        capture.pending_server = Some((Receive::Packet(packet::tests::ipv4()), (1, 3)));
        assert!(matches!(capture.receive(), Err(Error::InvalidPacket(_))));
        assert!(capture.pending_server.is_none());
    }

    #[test]
    fn malformed_or_backwards_native_timestamps_are_capture_errors() {
        for (seconds, micros) in [(-1, 0), (1, -1), (1, 1_000_000)] {
            let api = mock();
            let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
            STATE.with_borrow_mut(|state| {
                state.header.timestamp.tv_sec = seconds;
                state.header.timestamp.tv_usec = micros;
            });
            assert!(matches!(capture.server.receive(), Err(Error::InvalidPacket(_))));
        }
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
        STATE.with_borrow_mut(|state| state.header.timestamp.tv_sec = 2);
        assert!(matches!(capture.server.receive(), Ok(Receive::Packet(_))));
        STATE.with_borrow_mut(|state| state.header.timestamp.tv_sec = 1);
        assert!(matches!(capture.server.receive(), Err(Error::InvalidPacket(_))));
    }

    #[test]
    fn every_failed_setup_step_closes_and_never_exposes_capture() {
        for step in [
            "snaplen",
            "promiscuous",
            "timeout",
            "activate",
            #[cfg(unix)]
            "direction",
            "nonblock",
            "compile",
            "setfilter",
        ] {
            let api = mock();
            STATE.with_borrow_mut(|state| state.fail = step);
            assert!(api.open("test0", &[CLIENT]).is_err(), "{step}");
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
            let error = match api.open("test0", &[CLIENT]) {
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
            assert!(matches!(api.open(interface, &[CLIENT]), Err(Error::InvalidInput(_))));
        }
        STATE.with_borrow(|state| assert!(state.calls.is_empty()));
    }

    #[test]
    fn null_handle_and_unsupported_link_type_are_errors() {
        let api = mock();
        STATE.with_borrow_mut(|state| state.fail = "create");
        assert!(matches!(api.open("test0", &[CLIENT]), Err(Error::Native { .. })));
        STATE.with_borrow(|state| assert_eq!(state.closed, 0));

        let api = mock();
        STATE.with_borrow_mut(|state| state.link_type = 999);
        assert!(matches!(api.open("test0", &[CLIENT]), Err(Error::UnsupportedLinkType(999))));
        STATE.with_borrow(|state| assert_eq!(state.closed, 1));
    }

    #[cfg(unix)]
    #[test]
    fn receive_returns_only_typed_local_client_controls_and_rejects_payload() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
        for flags in [0x02, 0x10, 0x04, 0x14, 0x01, 0x11] {
            let bytes = packet::tests::client_ipv4(flags);
            let expected = packet::client_control(&bytes).expect("synthetic metadata");
            STATE.with_borrow_mut(|state| {
                state.bytes = bytes;
                state.header.captured_length = 40;
                state.header.original_length = 40;
            });
            assert_eq!(
                capture.client.receive().expect("metadata"),
                Receive::ClientControl(expected)
            );
            STATE.with_borrow_mut(|state| state.bytes[12] = 193);
            assert_eq!(capture.client.receive().expect("foreign source"), Receive::Discarded);
            STATE.with_borrow_mut(|state| {
                state.bytes[12] = 192;
                state.bytes.push(0x5a);
                state.bytes[2..4].copy_from_slice(&41_u16.to_be_bytes());
                state.header.captured_length = 41;
                state.header.original_length = 41;
            });
            assert_eq!(capture.client.receive().expect("client payload"), Receive::Discarded);
        }
    }

    #[cfg(unix)]
    #[test]
    fn cooked_link_direction_must_agree_with_control_or_server_variant() {
        for (kind, length, direction_offset) in [(113, 16, 1), (276, 20, 10)] {
            let api = mock();
            STATE.with_borrow_mut(|state| state.link_type = kind);
            let mut capture = api.open("test0", &[CLIENT]).expect("mock open");
            for outbound in [false, true] {
                for control in [false, true] {
                    let ip = if control {
                        packet::tests::client_ipv4(0x02)
                    } else {
                        packet::tests::ipv4()
                    };
                    let mut frame = vec![0; length];
                    let protocol_offset = if kind == 113 { 14 } else { 0 };
                    frame[protocol_offset..protocol_offset + 2].copy_from_slice(&[8, 0]);
                    frame[direction_offset] = if outbound { 4 } else { 0 };
                    frame.extend(ip);
                    STATE.with_borrow_mut(|state| {
                        state.header.captured_length = frame.len() as u32;
                        state.header.original_length = frame.len() as u32;
                        state.bytes = frame;
                    });
                    let result =
                        if control { capture.client.receive() } else { capture.server.receive() }
                            .expect("mock receive");
                    assert_eq!(matches!(result, Receive::Discarded), outbound != control);
                }
            }
        }
    }

    #[test]
    fn receive_handles_idle_end_error_and_rejects_client_packets() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");

        for (status, expected) in [(0, Receive::Idle), (-2, Receive::End)] {
            STATE.with_borrow_mut(|state| state.next_status = status);
            assert_eq!(capture.server.receive().expect("non-packet"), expected);
        }

        STATE.with_borrow_mut(|state| state.next_status = -1);
        assert!(matches!(capture.server.receive(), Err(Error::Native { .. })));

        STATE.with_borrow_mut(|state| {
            state.next_status = 1;
            state.bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
        });
        assert_eq!(capture.server.receive().expect("client discarded"), Receive::Discarded);
    }

    #[test]
    fn receive_admits_both_server_ports_and_rejects_both_client_directions() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");

        for server in [3101_u16, 5222] {
            STATE.with_borrow_mut(|state| {
                state.bytes[20..22].copy_from_slice(&server.to_be_bytes());
                state.bytes[22..24].copy_from_slice(&45000_u16.to_be_bytes());
            });
            let expected = STATE.with_borrow(|state| state.bytes.clone());
            assert_eq!(capture.server.receive().expect("server packet"), Receive::Packet(expected));

            STATE.with_borrow_mut(|state| {
                state.bytes[20..22].copy_from_slice(&45000_u16.to_be_bytes());
                state.bytes[22..24].copy_from_slice(&server.to_be_bytes());
            });
            assert_eq!(capture.server.receive().expect("client packet"), Receive::Discarded);

            for destination in [3101_u16, 5222] {
                STATE.with_borrow_mut(|state| {
                    state.bytes[20..22].copy_from_slice(&server.to_be_bytes());
                    state.bytes[22..24].copy_from_slice(&destination.to_be_bytes());
                });
                assert_eq!(capture.server.receive().expect("ambiguous pair"), Receive::Discarded);
            }
        }
    }

    #[test]
    fn receive_checks_lengths_and_null_outputs_before_dereferencing_data() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");

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
            assert!(matches!(capture.server.receive(), Err(Error::InvalidPacket(_))));
        }
    }

    #[test]
    fn supported_link_headers_are_bounded_and_cooked_direction_is_preserved() {
        let ip = packet::tests::ipv4();
        for (kind, mut header) in [
            (1, vec![0; 14]),
            (113, vec![0; 16]),
            (276, vec![0; 20]),
            (0, LOOP_IPV4.to_ne_bytes().to_vec()),
            (108, LOOP_IPV4.to_be_bytes().to_vec()),
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
                assert_eq!(link_packet(kind, &header), Some(ip.as_slice()));
                assert_eq!(cooked_direction(kind, &header), Some(true));
                header[1] = 1;
                assert_eq!(link_packet(kind, &header), None);
            }

            if kind == 276 {
                header[10] = 4;
                assert_eq!(link_packet(kind, &header), Some(ip.as_slice()));
                assert_eq!(cooked_direction(kind, &header), Some(true));
                header[10] = 1;
                assert_eq!(link_packet(kind, &header), None);
            }
        }
    }

    #[test]
    fn client_allowlist_is_bounded_deduplicated_and_parenthesized() {
        let ipv6: IpAddr = "2001:db8::2".parse().expect("test IP");
        let (clients, filter) =
            client_filter(&[CLIENT, ipv6, CLIENT], true).expect("valid clients");
        assert_eq!(clients, [CLIENT, ipv6]);
        let filter = filter.to_str().expect("ASCII");
        assert!(!filter.contains("dst host"));
        let (_, server_filter) =
            client_filter(&[CLIENT, ipv6, CLIENT], false).expect("server filter");
        assert!(
            server_filter
                .to_str()
                .expect("ASCII")
                .contains("and (dst host 192.0.2.2 or dst host 2001:db8::2)")
        );
        assert!(filter.contains("and (src host 192.0.2.2 or src host 2001:db8::2)"));
        assert!(filter.contains("ip[2:2] = ((ip[0] & 15) * 4 + (tcp[12] & 240) / 4)"));
        assert!(filter.contains("ip6[4:2] = (ip6[52] & 240) / 4"));

        let api = mock();
        for clients in [
            vec![],
            vec![CLIENT; 17],
            vec![IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)],
            vec!["ff02::1".parse().expect("multicast")],
            vec!["255.255.255.255".parse().expect("broadcast")],
        ] {
            assert!(matches!(api.open("test0", &clients), Err(Error::InvalidInput(_))));
        }
        STATE.with_borrow(|state| assert!(state.calls.is_empty()));
    }

    #[test]
    fn packet_destination_cannot_escape_configured_client_scope() {
        let api = mock();
        let mut capture = api.open("test0", &[CLIENT]).expect("mock open");

        // The server port alone is insufficient: this packet heads away from the
        // configured client. Windows has no pcap_setdirection safety net.
        STATE.with_borrow_mut(|state| state.bytes[16..20].copy_from_slice(&[203, 0, 113, 1]));
        assert_eq!(capture.server.receive().expect("mock receive"), Receive::Discarded);

        STATE.with_borrow_mut(|state| state.bytes[16..20].copy_from_slice(&[192, 0, 2, 2]));
        assert!(matches!(capture.server.receive(), Ok(Receive::Packet(_))));
    }

    #[test]
    fn ipv6_destination_admission_and_loopback_family_are_exact() {
        let client: IpAddr = "2001:db8::2".parse().expect("test IP");
        let mut ipv6 = vec![0; 61];
        ipv6[0] = 0x60;
        ipv6[4..6].copy_from_slice(&21_u16.to_be_bytes());
        ipv6[6] = 6;
        if let IpAddr::V6(address) = client {
            ipv6[24..40].copy_from_slice(&address.octets());
        }
        ipv6[40..].copy_from_slice(&packet::tests::ipv4()[20..]);
        assert_eq!(packet::destination(&ipv6), Some(client));
        assert!(packet::server_packet(&ipv6).is_some());

        let mut frame = LOOP_IPV6.to_ne_bytes().to_vec();
        frame.extend_from_slice(&ipv6);
        assert_eq!(link_packet(0, &frame), Some(ipv6.as_slice()));

        #[cfg(windows)]
        {
            assert_eq!(LOOP_IPV6, 24); // Npcap BSD wire value, not Winsock's 23
            frame[..4].copy_from_slice(&23_u32.to_ne_bytes());
            assert_eq!(link_packet(0, &frame), None);
        }
    }

    #[test]
    fn pcap_header_layout_matches_native_64_bit_platform_abi() {
        #[cfg(windows)]
        {
            assert_eq!(std::mem::size_of::<Timeval>(), 8);
            assert_eq!(std::mem::size_of::<PacketHeader>(), 16);
            assert_eq!(std::mem::align_of::<PacketHeader>(), 4);
            assert_eq!(std::mem::offset_of!(PacketHeader, captured_length), 8);
            assert_eq!(std::mem::offset_of!(PacketHeader, original_length), 12);
        }

        #[cfg(unix)]
        {
            assert_eq!(std::mem::size_of::<PacketHeader>(), 24);
            assert_eq!(std::mem::offset_of!(PacketHeader, captured_length), 16);
            assert_eq!(std::mem::offset_of!(PacketHeader, original_length), 20);
        }

        assert_eq!(std::mem::size_of::<BpfProgram>(), 16);
        assert_eq!(std::mem::offset_of!(BpfProgram, instructions), 8);
    }

    #[cfg(windows)]
    #[test]
    fn failed_utf8_initialization_never_creates_a_handle() {
        let api = mock();
        STATE.with_borrow_mut(|state| state.fail = "init");
        assert!(matches!(
            api.open("test0", &[CLIENT]),
            Err(Error::Native { operation: "pcap_init", detail }) if detail == "mock UTF8 initialization failure"
        ));
        STATE.with_borrow(|state| {
            assert_eq!(state.calls, ["init"]);
            assert_eq!(state.closed, 0);
        });
    }

    #[cfg(windows)]
    #[test]
    fn initialization_is_serialized_and_repeatable_without_direction_symbol() {
        let api = mock();
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let api = &api;
                scope.spawn(move || {
                    for _ in 0..4 {
                        let capture = api.open("test0", &[CLIENT]).expect("mock open");
                        drop(capture);
                    }
                    STATE.with_borrow(|state| {
                        assert_eq!(state.calls.iter().filter(|call| **call == "init").count(), 4);
                        assert!(!state.calls.contains(&"direction"));
                        assert_eq!(state.closed, 4);
                    });
                });
            }
        });
    }
}
