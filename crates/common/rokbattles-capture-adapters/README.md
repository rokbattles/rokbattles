# Passive capture adapters

An isolated library aligned with the six targets in the existing release matrix:

- Windows x86_64: WinDivert; pcap (Npcap) remains server-packet-only
- Windows aarch64 (ARM64): pcap server-packet-only; WinDivert returns `UnsupportedPlatform`
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

## Passive capture and local control metadata

- WinDivert's fixed filter selects inbound TCP source ports 3101/5222 plus
  outbound zero-payload TCP SYN, ACK, FIN or RST to those destination ports.
  Server-to-server traffic is excluded. Its fixed flags remain
  `SNIFF | RECV_ONLY | NO_INSTALL`: no packet injection, network modification,
  driver installer or elevation helper is included.
- pcap is non-promiscuous and nonblocking. Its fixed BPF selects server packets
  to the typed local client address allowlist, and zero-payload client controls
  from that same allowlist. IPv4 options and TCP options are included when
  calculating payload length. Plain IPv6 uses explicit IP-relative offsets;
  extension headers and fragments are rejected. Unix opens separate `PCAP_D_IN`
  and `PCAP_D_OUT` handles with disjoint server/control filters. Either direction
  failing closes both handles; addresses alone never prove outbound direction.
  Npcap does not load or call its unsupported direction API and never returns
  client controls. `Pcap::client_controls_supported()` is false on Windows;
  callers must require another authenticated control source before starting
  a handshake-gated lifecycle there. No unfiltered fallback is exposed.
- Both adapters independently parse packet bounds and compute the full TCP
  payload length before admitting client metadata. SYN/FIN/RST flags may coexist
  with payload, so flags alone never qualify a packet. A client packet containing
  even one payload byte is rejected. Client packets are never returned or copied
  to the application; `Receive::ClientControl(ClientTcpControl)` contains only
  typed endpoints, sequence/acknowledgement numbers and flags. Its Debug output
  is redacted. Server packets continue to use `Receive::Packet`.
- pcap independently verifies local destination/source addresses. WinDivert
  independently requires inbound direction for server packets and outbound for
  client controls and rejects the native impostor flag. Linux SLL/SLL2 additionally
  require that their direction agrees with the admitted variant. Unknown/multicast
  cooked-link types are rejected. Inbound packets with spoofed local source IPs
  cannot enter the outbound-only native handle or its client metadata path.
- The packet gate accepts complete unfragmented IPv4 and base-header IPv6 TCP.
  IPv6 extension headers, jumbograms, fragments, truncated records, VLAN frames,
  and unsupported link types are rejected. pcap supports Ethernet, raw IP,
  NULL/LOOP, Linux SLL/SLL2, and explicit IPv4/IPv6 link types.
- WinDivert `receive` is synchronous and blocking. Use a capture thread; another
  thread can call `shutdown` to unblock it. Queued packets drain before `End`.
  pcap returns `Idle` without blocking. Its Unix wrapper polls both handles and
  merges one pending typed result per direction by native timestamp, choosing
  client controls first on ties. Pending client state contains metadata only.
  Invalid/backwards timestamps and ordering regressions fail capture rather than
  reordering lifecycle evidence silently. Neither starts background services.

## Connection probes and lifecycle boundary

The unprivileged runtime's `lifecycle::Observer::client` validates local control
metadata and maps it to the same bounded generations used by `Observer::server`.
It must observe the client SYN, matching empty server SYN-ACK, and exact final
zero-payload client ACK before emitting `Open`. Capture starting midstream or
missing handshake metadata cannot promote a stream. All state remains bounded
by 128 generations, two per tuple, candidate/active idle expiry, and existing
server reassembly limits; active connections have no short lifetime limit.

A reset must match exact client sequence evidence from the witnessed generation
(last zero-payload control or server acknowledgement). RST with ACK must also
match that generation's server sequence window. Unassignable stale controls are
discarded; ambiguous matches abort every matched generation rather than guessing
the newest. An observed probe reset therefore retires its own generation and
late server `SxNtf` bytes cannot enter a stream. Independent simultaneous tuples
remain independent. Client payload is neither needed nor admitted to track
server acknowledgements after unseen client data.

Client FIN consumes one sequence number and is a half-close: server data can
continue. Out-of-order server FIN waits for missing preceding server bytes;
bytes past a witnessed FIN, capture gaps, and invalid handshakes abort rather
than stitch streams. Metadata and connection addresses remain local; the ingress
wire protocol is unchanged and has no client metadata variant or encoder.

An observation gap, source switch, queue drop, or broken local IPC requires
`Observer::gap()` before more data, and reopening requires a fresh handshake.
A zero-payload-only policy cannot observe client controls that carry payload;
missing controls and native packet loss remain observation limits, not permission
to expand capture. Live/native filter behavior remains unvalidated here.

## Scope

The adapters perform no protocol decoding/reassembly, ingress, persistence, SQLite,
background service, IPC, driver installation, UI or packaging. The adapters do not log or persist packet bytes. `Receive` Debug shows only the
server byte count or a redacted control variant. WinDivert's owned native receive
buffer is zeroized on every return path, including rejection and errors. Pending
owned Unix server packets are wiped when the paired capture is dropped. Callers
must protect/zeroize the owned server packet after `receive` transfers it. pcap
returns a library-owned const buffer; this crate never mutates that memory and
cannot guarantee the native library's clearing behavior. Rejected pcap bytes are
never copied into application-owned storage.

## Safe verification

```sh
cargo test --locked -p rokbattles-capture-adapters -p rokbattles-capture-runtime
cargo clippy --locked -p rokbattles-capture-adapters -p rokbattles-capture-runtime --all-targets -- -D warnings
cargo fmt -p rokbattles-capture-adapters -p rokbattles-capture-runtime -- --check
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
[Npcap filter arithmetic and IPv6 limits](https://npcap.com/guide/wpcap/pcap-filter.html),
[Npcap initialization](https://npcap.com/guide/wpcap/pcap_init.html),
[Npcap direction limitation](https://github.com/nmap/npcap/issues/248).
