# Passive capture adapters

An isolated library providing WinDivert on Windows x86/x64 and libpcap on Linux
and macOS. The desktop application and existing services do not depend on it,
load it, or start capture. Other targets return `UnsupportedPlatform` from `load`.

## Explicit lifecycle and native dependencies

1. Select an administrator-controlled, architecture-matching native library path.
2. Explicitly call `unsafe WinDivert::load(path)` or `unsafe Pcap::load(path)`.
   The path must be absolute and is canonicalized. The caller must establish
   binary/dependency trust and the documented native ABI. Loading native code
   executes its initializers; path canonicalization alone cannot establish trust.
3. Explicitly call `open` only when capture has been authorized. `load` never
   opens a capture handle. pcap takes a local interface name, not an rpcap URL.
4. Keep the loaded backend alive while using its borrowed `Capture` handle.
   Private function pointers cannot outlive the retained `libloading::Library`.
   Each handle closes exactly once, including failures during setup.

There is no build script, native import library, pcap crate, or mandatory libpcap
link dependency. A machine without the native library can build and run the
application and these unit tests. Missing binaries, incompatible/missing symbols,
unsupported platforms, permission errors, and native failures return errors.

WinDivert uses its 2.x **C/cdecl ABI**, including on 32-bit Windows; its address
buffer is 80 bytes with 8-byte alignment. Windows library dependencies are
restricted to System32, with no PATH or working-directory fallback. Unix dynamic
loaders still resolve the selected library's dependencies: privileged callers
must use trusted installations and a sanitized loader environment.

## Passive, one-direction contract

- WinDivert's fixed filter selects inbound TCP source port 3101 and excludes
  destination port 3101. Its fixed flags are `SNIFF | RECV_ONLY | NO_INSTALL`.
  Original packets are not diverted/dropped, no injection symbol is loaded, and
  a driver which is not already installed is an error. No driver installer or
  elevation/security-setting helper is included.
- pcap is non-promiscuous, uses a fixed server-source BPF filter, and requires
  `PCAP_D_IN`. Failure to enforce inbound direction closes the handle; it never
  falls back to bidirectional capture. The filter must install successfully
  before any capture is exposed. Reads are nonblocking.
- Both adapters independently validate packet bounds and TCP source/destination
  ports before returning an owned IP packet. Client-originated packets never
  leave the adapter. Server SYN/FIN/RST packets without payload are retained.
  Ambiguous port-3101-to-port-3101 traffic is excluded conservatively.
- The small admission check accepts unfragmented IPv4 and base-header IPv6 TCP.
  IPv6 extension headers, jumbograms, fragments, truncated records, VLAN frames,
  and unsupported link types are rejected rather than guessed or reassembled.
  pcap supports Ethernet, raw IP, NULL/LOOP, Linux SLL/SLL2, and explicit IPv4/IPv6
  link types. These deliberate limitations can lose traffic, never widen capture.
- WinDivert `receive` is synchronous and blocking. Use a capture thread; another
  thread can call `shutdown` through a shared handle reference to unblock it.
  Queued packets drain before `End`. pcap returns `Idle` without blocking so the
  caller can choose scheduling/cancellation. There is no background service here.

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
CI runs the mocks on Linux, macOS, Windows x64, and Windows x86 without installing
libpcap or WinDivert. Live permissions, driver compatibility, actual NIC/link-layer
behavior, packet delivery and shutdown timing remain **unvalidated**; checking
these later needs explicit authorization on the target operating systems.

ABI references: [WinDivert 2.2 header](https://github.com/basil00/WinDivert/blob/v2.2.2/include/windivert.h),
[WinDivert API](https://reqrypt.org/windivert-doc.html),
[libpcap public header](https://github.com/the-tcpdump-group/libpcap/blob/libpcap-1.10.5/pcap/pcap.h).
