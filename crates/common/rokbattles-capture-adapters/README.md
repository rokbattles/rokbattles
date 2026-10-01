# Passive capture adapters

An isolated library aligned with the six targets in the existing release matrix:

- Windows x86_64: WinDivert and pcap (Npcap)
- Windows aarch64 (ARM64): pcap only; WinDivert returns `UnsupportedPlatform`
- macOS x86_64 (Intel) and aarch64 (Apple Silicon): pcap
- Linux x86_64 and aarch64 (ARM64): pcap

32-bit Windows is not supported. Other OS/architecture combinations also return
`UnsupportedPlatform`. The desktop app and existing services do not depend on
this crate, load its libraries, or start capture. No release/packaging file changes
are needed for this matrix.

## Explicit lifecycle and native dependencies

1. Select an administrator-controlled, architecture-matching native library path.
2. Explicitly call `unsafe WinDivert::load(path)` or `unsafe Pcap::load(path)`.
   The path must be absolute and is canonicalized. The caller must establish
   binary/dependency trust and the documented native ABI. Loading native code
   executes its initializers; path canonicalization alone cannot establish trust.
3. Explicitly call `open` only when capture has been authorized. `load` never
   opens a capture handle. pcap takes a local interface name (not an rpcap URL)
   and 1..=16 typed, current local client IP addresses for that interface. Address
   discovery and refreshing are caller responsibilities in a later integration.
   Empty, oversized, unspecified, multicast, and broadcast lists fail before
   native calls; duplicates are removed. Reopen after addresses change.
4. Keep the loaded backend alive while using its borrowed `Capture` handle.
   Private function pointers cannot outlive the retained `libloading::Library`.
   Each handle closes exactly once, including failures during setup.

There is no build script, native import library, pcap crate, or mandatory libpcap
link dependency. A machine without the native library can build and run the
application and these unit tests. Missing binaries, incompatible/missing symbols,
unsupported platforms, permission errors, and native failures return errors.

WinDivert uses its 2.x C ABI; its address buffer is 80 bytes with 8-byte alignment.
Windows library dependencies are restricted to the selected trusted DLL directory
and System32, with no PATH or working-directory fallback or process-wide search
path changes. This admits Npcap's side-by-side Packet.dll. Unix dynamic loaders
still resolve the selected library's dependencies: privileged callers must use
trusted installations and a sanitized loader environment.

Npcap uses a Windows pcap header with two 32-bit timestamp fields (16-byte header),
not Unix's 24-byte header. Windows requires `pcap_init(PCAP_CHAR_ENC_UTF_8)` before
`pcap_create`, disabling the legacy unsafe UTF16 probe. Initialization is serialized
by a mutex, can repeat across DLL unload/reload, and fails closed. The unsafe
loader contract excludes concurrent initialization by callers outside this crate.
Unix keeps its platform libc timeval and does not require this newer init symbol.
No Npcap binaries, drivers, or installers are redistributed by this crate.

## Passive, one-direction contract

- WinDivert's fixed filter selects inbound TCP source ports 3101 or 5222 and
  excludes either port as a destination. Its fixed flags are `SNIFF | RECV_ONLY | NO_INSTALL`.
  Original packets are not diverted/dropped, no injection symbol is loaded, and
  a driver which is not already installed is an error. No driver installer or
  elevation/security-setting helper is included.
- pcap is non-promiscuous and nonblocking. Its fixed server-source/port grammar
  selects TCP source ports 3101 or 5222, excludes both destination ports, and
  is narrowed by a parenthesized destination allowlist built only from typed IPs.
  Unix additionally requires `PCAP_D_IN` and fails closed if it is unsupported.
  Npcap does not implement `pcap_setdirection`, so Windows neither loads nor calls
  it: the destination BPF plus an independent destination-address admission check
  enforce server-to-local-client direction. No broader unfiltered fallback is
  exposed, and filter installation must succeed before returning a capture.
- Both adapters independently validate packet bounds and TCP source/destination
  ports before returning an owned IP packet; pcap also checks the client IP.
  Client-originated packets never leave the adapter. Server SYN/FIN/RST packets without payload are retained.
  Ambiguous traffic between any pair of the two server ports is excluded
  conservatively, including 3101-to-5222 and 5222-to-3101.
- The small admission check accepts unfragmented IPv4 and base-header IPv6 TCP.
  IPv6 extension headers, jumbograms, fragments, truncated records, VLAN frames,
  and unsupported link types are rejected rather than guessed or reassembled.
  pcap supports Ethernet, raw IP, NULL/LOOP, Linux SLL/SLL2, and explicit IPv4/IPv6
  link types. These deliberate limitations can lose traffic, never widen capture.
- WinDivert `receive` is synchronous and blocking. Use a capture thread; another
  thread can call `shutdown` through a shared handle reference to unblock it.
  Queued packets drain before `End`. pcap returns `Idle` without blocking so the
  caller can choose scheduling/cancellation. There is no background service here.

## Connection probes and lifecycle boundary

These adapters validate each packet independently. They retain valid server
segments from both game ports, including an isolated SYN-ACK or a late `SxNtf`
from a short-lived connection probe. They do not classify probes, remember active
connections, or decide whether a notification belongs to a live game session.
Server-only capture cannot observe the client's reset, so this API cannot reliably
reject a server packet solely because the client has already sent RST.

A future connection/lifecycle layer should:

- Track each client/server address-and-port tuple independently, with a connection
  generation to distinguish reused tuples. The game's simultaneous connections
  must not suppress each other merely because another connection is active.
- Classify provisional probes using connection history and protocol evidence,
  with explicit bounded buffering and expiry. A handshake, one packet, or a short
  duration alone does not prove a connection is disposable. Define handling for
  capture that starts mid-connection, missing packets, and reordered delivery.
- Use validated lifecycle evidence to discard notifications for an aborted
  connection generation. Detecting a client-originated reset requires additional
  client control metadata or equivalent local socket-state evidence. Any future
  adapter extension must return typed header metadata only (endpoints, TCP flags,
  sequence/acknowledgment numbers and observation ordering), never client payload
  bytes; flags can coexist with payload, so flags alone cannot enforce that rule.
- Distinguish reset from orderly close and preserve valid data carried with FIN.
  FIN closes one direction; it does not mean the opposite direction is closed.
  See [TCP close and half-close semantics](https://www.rfc-editor.org/rfc/rfc9293.html#section-3.6).

These are requirements for a separately scoped stage. This crate does not widen
capture to client packets, expose client control metadata, or implement a tracker.
Probe promotion criteria and buffering limits remain to be specified there.

## Scope

No protocol decoding/reassembly, ingress, persistence, SQLite, background service,
IPC, driver installation, UI, desktop integration, or packaging. Later integration
is a separate stage. The adapters do not log or persist packet bytes.

## Safe verification

```sh
cargo test --locked -p rokbattles-capture-adapters
cargo clippy --locked -p rokbattles-capture-adapters --all-targets -- -D warnings
cargo fmt -p rokbattles-capture-adapters -- --check
```

Tests use synthetic packets and private mocked native function tables. The
missing-symbol test queries only the already-running test executable. Tests do
not open real interfaces, load capture libraries/drivers, or request privileges.
CI runs the mocks natively on all six release targets (Windows x64/ARM64,
macOS Intel/Apple Silicon, Linux x64/ARM64) without installing libpcap, Npcap,
or WinDivert. Cross-compilation checks are supplemental, not native runtime tests. Live permissions, driver compatibility, actual NIC/link-layer
behavior, packet delivery and shutdown timing remain **unvalidated**; checking
these later needs explicit authorization on the target operating systems.

ABI references: [WinDivert 2.2 header](https://github.com/basil00/WinDivert/blob/v2.2.2/include/windivert.h),
[WinDivert API](https://reqrypt.org/windivert-doc.html),
[libpcap public header](https://github.com/the-tcpdump-group/libpcap/blob/libpcap-1.10.5/pcap/pcap.h),
[Npcap development guide](https://npcap.com/guide/npcap-devguide.html),
[Npcap initialization](https://npcap.com/guide/wpcap/pcap_init.html),
[Npcap direction limitation](https://github.com/nmap/npcap/issues/248).
