# Desktop capture: review and local testing

The desktop UI, unprivileged background agent, privileged capture helper and
server decoder have separate responsibilities. Closing the UI leaves the agent
running; Pause, Stop and capture consent control its work through private SQLite
state. The helper has no database, HTTP client, decoder, upload credential or
user-selected filter, library, path or command.

## Platform behavior

| Native desktop package | Capture preference | Without capture prerequisites |
| --- | --- | --- |
| Windows x64 | Bundled WinDivert, then separately installed Npcap | Mailcache |
| Windows ARM64 | Separately installed Npcap | Mailcache |
| macOS Intel / Apple Silicon | System libpcap and installed signed helper | Mailcache |
| Linux x64 / ARM64 | System libpcap and installed helper | Mailcache |
| Flatpak | Capture unavailable | Portal-accessible mailcache |

Npcap is never downloaded, installed or redistributed by the application.
WinDivert's pinned official release has no native ARM64 driver. Missing libraries
are recoverable errors: the adapters use retained `libloading` handles without a
mandatory pcap/WinDivert link dependency. Loading a native library executes code;
only fixed protected installations with the expected ABI are eligible.

The Windows installer packages the official hash-pinned unsigned WinDivert DLL
and upstream-signed SYS together. Preserve the vendor license, attribution and
corresponding-source notices. See the [vendor build instructions](../crates/apps/rokbattles-desktop/vendor/README.md).

## Data and consent

Capture is off by default. For opted-in connections owned by the OS account,
server-to-client TCP bytes from ports 3101 and 5222 go to the fixed HTTPS capture
endpoint for protocol classification. Other applications can use these ports;
port selection alone does not identify Rise of Kingdoms. Only recognized game
mail is stored. Non-mail messages and encrypted stream bytes are not persisted.

Client payload is excluded. Outbound controls are admitted only when their full
TCP payload length is zero, and their addresses, flags and sequence numbers stay
local. A control carrying even one payload byte is rejected. Filters and the
independent packet parser enforce the same port and direction rules. Neither
adapter loads an injection function or modifies original game traffic.

The server retains the stream cipher, incomplete frame and player/server hints
in memory while decoding incrementally. It stores completed mails as they arrive
and drops connection state after disconnect, loss or expiry. Context hints are
not authentication or ownership evidence. The anonymous capture endpoint is
physically absent unless `DESKTOP_CAPTURE_ENABLED` is explicitly enabled; it
never accepts a desktop copy of `RELAY_TOKEN`. Deployment still requires the
private server artifact, compatible unbuffered proxy behavior and edge limits.

SQLite contains settings, mailcache paths, file signatures and bounded queue
state, not a captured-stream spool. It is plaintext protected by the OS user's
private directory, file permissions and the Windows DACL where applicable.
Ordinary deletion is not secure erasure. Existing JSON history is not imported;
select directories again. This is an unreleased SQLite schema with no migration
path from interim development databases.

## Connection and loss handling

- WinDivert and Unix pcap require witnessed client SYN, server SYN/ACK and final
  zero-payload client ACK. Existing or incompletely observed connections remain
  excluded until a new handshake. Independent tuples and bounded reused-tuple
  generations are tracked separately; there is no global single-active rule.
- Windows Npcap uses separate OS socket-established/retired evidence. Existing
  socket rows are excluded at startup. A new row's creation identity, pinned
  process/logon, SYN_SENT-to-ESTABLISHED transition and server SYN/ACK must agree.
  It never fabricates client control packets. Missed transitions are a fallback
  limitation, not permission to guess a generation.
- Ownership is rechecked before each helper record is sent. A changed or
  ambiguous process/socket identity fails closed. Client FIN is a half-close;
  valid server tail bytes remain eligible. Reordered server FIN waits for earlier
  bytes. RST, gaps, invalid overlap and bytes beyond FIN cannot stitch streams.
- Packet normalization rejects fragments, IPv6 extension headers/jumbograms,
  VLAN and unsupported/truncated link records. Client controls carrying payload,
  native packet loss and OS ownership polling can make a connection unobservable.
- Reassembly, generations, decoder frames, IPC queues, HTTP queues and decoding
  work have independent bounds. Initial/partial-frame deadlines and stale-flow
  expiry apply; active connections have no short fixed ten-minute cutoff.
- A pre-handshake orphan can become a bounded, expiring sink without disrupting
  another valid flow. A protocol failure after any completed handshake poisons
  the transport. Client metadata has no ingress wire representation.
- Queue overflow, capture/IPC loss, backend switch, malformed framing or an early
  HTTP response closes the observer and upload together. There is no ciphertext
  replay or midstream resume. Reconnect the game to create fresh evidence. Mails
  stored before an interruption remain stored; failure is not transactional.

Owned raw buffers and cipher state are wiped on retirement, and Debug/errors do
not print payloads. Native pcap owns its receive buffer; the application cannot
promise erasure of that library-owned memory. On Unix, helper and agent enforce
zero process-local core limits before opening state or processing traffic; Linux
also disables dumpability for piped core collectors. Tests inject this policy
rather than modifying the test process or host.

## Installation and update boundaries

Windows uses per-machine protected Program Files paths. Local named pipes use
first-instance protection, remote-client rejection and SYSTEM plus the exact
interactive logon SID. Both ends verify the expected process/service identity.
The service accepts only the bounded Start/Stop capture protocol. Runtime opens
WinDivert with `SNIFF | RECV_ONLY | NO_INSTALL`; explicit maintenance owns driver
registration/start, serialized with all application opens. This serialization
does not control unrelated third-party WinDivert callers.

The installer holds its lifetime lock through quiescence, verified backup,
replacement, readiness and maintenance-image promotion. It verifies existing
service types and exact protected ImagePaths before changing them, preserving
conflicting third-party services. All old executable/DLL/SYS images are pinned
before any is retired; busy images abort replacement and can require reboot.
The helper stops before the driver. Backup manifests include absent files so
rollback does not leave newly introduced binaries behind.

Maintenance performs online whole-chain driver revocation checks against the
pinned nested Microsoft signature. Runtime verifies that same cryptographic
driver policy offline, plus exact size/hash, protected paths and the running
kernel-service image. Runtime has no network revocation/cache dependency;
revocation freshness belongs to maintenance and release updates. Neither path
ignores signature errors, changes trust stores or enables test signing. Hash
authentication of application executables is not a claim of app Authenticode
signing.

The protected maintenance marker blocks helper opens and requests agent exit;
the unprivileged updater also records a durable stop request without changing the
desired enabled setting. Elevated code never reads the user database. The marker
is removed only after verified completion. Failed/interrupted repair remains
blocked, preserves recovery evidence and never overwrites a running driver.

Unix uses a fixed root-protected companion and kernel-authenticated local IPC.
Missing setup leaves the bundled mailcache worker available; a present untrusted
companion fails closed. A bundled agent cannot authenticate in place of the
protected image. macOS also verifies the exact signed/hardened helper and agent
identities, team and running/static code hashes. Unsigned debug builds cannot
receive capture. Do not run the UI or mailcache agent as root.

The [Unix package and administrator guide](../crates/apps/rokbattles-capture-helper/packaging/unix/README.md)
is also shipped in that platform's capture archive. Its installer requires a
release-bound payload and an independently authenticated, protected root-owned
script invoked by trusted Python in isolated mode. It is an administrator tool,
not a completed consumer installation experience. macOS needs the signed and
notarized distribution and trusted Python; SMAppService consumer setup is not
implemented. Do not broaden Flatpak access or device permissions as a workaround.

## Automated checks

From the repository root, after installing the repository's normal prerequisites:

```sh
pnpm -F @rokbattles/desktop-client test
pnpm -F @rokbattles/desktop-client build
cargo test --locked -p rokbattles-desktop-agent -p rokbattles-capture-ipc
cargo clippy --locked -p rokbattles-desktop-agent -p rokbattles-capture-ipc --all-targets -- -D warnings
python -m unittest discover -s crates/apps/rokbattles-capture-helper/packaging/unix -p 'test_*.py' -v
```

Capture Foundation runs synthetic/mock checks on six native targets, plus Unix
parser/storage regressions, Windows bootstrap/NSIS compilation and signature-only
positive/negative fixtures. Vendor checks validate exact archive members, hashes,
licenses and Windows driver policy. They do not install services, activate a
driver or capture traffic. The separate real-library workflow, once included in
the reviewed tree, builds the full desktop and resolves native library symbols;
loading a DLL still does not prove capture, privilege or installer behavior.

Keep existing workflows until their replacement contains every check and passes
on the exact replacement head. Results from another commit are supporting
evidence, not validation of a changed tree. Npcap runtime testing needs a separate
accepted/licensed installation; an SDK or mocked function table is insufficient.

## Native library checks

The native workflow builds the complete desktop on Windows, macOS and Ubuntu
x64/ARM64. It loads distribution-owned libpcap on all four Unix targets and the
verified official WinDivert DLL on Windows x64. Windows ARM64 has no WinDivert
runtime. These narrowly selected ignored tests only resolve adapter symbols:
they do not initialize pcap, enumerate interfaces, open capture, receive packets
or start a driver/service. Do not run every ignored test indiscriminately.

For an explicitly prepared trusted Unix runtime, from the repository root:

```sh
cargo test --locked -p rokbattles-capture-adapters --test native_dependencies pcap_system_library_loads_without_capture -- --ignored --exact
```

The Unix probe uses fixed Ubuntu architecture paths or Apple's exact
`/usr/lib/libpcap.A.dylib`, including dyld-cache-only systems. Clear loader override
variables before invoking it, as the workflow does. On Windows x64, use the
[vendor staging and signature checks](../crates/apps/rokbattles-desktop/vendor/README.md)
first, then select only `windivert_verified_library_loads_without_capture` with
`--ignored --exact`. Library initializers execute during loading; trusted provenance
is still required even when capture is never opened.

Npcap runtime is explicitly **NOT RUN / BLOCKED** in hosted CI. Its free installer
is interactive; [silent installation requires Npcap OEM](https://npcap.com/guide/npcap-users-guide.html).
An SDK/import library is not a runtime installation. CI does not download, cache,
extract, install or redistribute any Npcap component or accept its terms.

On a separately authorized Windows x64/ARM64 machine whose administrator already
accepted the license and installed Npcap, use native PowerShell 7:

```powershell
./.github/native-capture/probe-installed-npcap.ps1
```

This script accepts no path or installer URL. It checks native architecture,
fixed System32/Npcap DLLs and valid Nmap Software LLC publisher signatures, then
runs the exact load-only test. Missing/untrusted libraries fail explicitly.
Do not expose a persistent self-hosted machine to untrusted pull-request code.

## Local acceptance checklist

Use a disposable test installation and record the exact commit, OS/architecture,
backend and signed artifact version. These are manual checks for the tester;
the test suites above do not perform privileged installation or live uploads.

1. **Mailcache first:** start without capture prerequisites. Select a test
   mailcache directory, close and reopen the UI, then pause, resume and stop the
   worker. Confirm one agent, bounded retries and no lost pending work. Review
   which test mails may be uploaded before enabling it.
2. **Consent:** capture starts off. Verify enable/disable, pause and maintenance
   stop affect an already-running agent. An unsupported package must still allow
   existing consent to be revoked. Flatpak must report mailcache-only operation.
3. **Trusted setup:** use the matching reviewed installer/package. On Windows,
   confirm the one explicit administrator flow, protected paths, official DLL/SYS
   pair and serialized driver startup. Test ARM64 without WinDivert. Install
   Npcap separately only if accepting its terms and testing that fallback.
4. **Capture session:** after the test endpoint and server artifact are ready,
   start a fresh game connection. Confirm mail arrival before game disconnect,
   independent concurrent flows, probe/reset isolation and server tail delivery
   after client half-close. Client payload and controls must never enter uploads.
5. **Loss and revocation:** disconnect IPC/network, stop the helper and revoke
   consent. Confirm bounded shutdown, no stalled game traffic, no stream replay
   and a new handshake required before capture resumes. Earlier stored mails may
   remain. Test a long active game session and a long idle interval.
6. **Account boundary:** test a second OS user and logout. One user's agent must
   not receive another user's records or reuse that user's socket generation.
7. **Maintenance:** test install/update/cancel/installer death, occupied images,
   reboot-required behavior, repair, rollback and final-user uninstall. Confirm
   markers persist through failure and unrelated services/files are preserved.
8. **Privacy and finish:** inspect only status/error codes in logs, verify raw
   streams are absent from disk and check the endpoint remains off outside the
   intended test deployment. Report failures with commit/platform/backend and
   stable error codes; do not attach packet dumps, mail bodies or credentials.

Successful compilation, mocks and signature checks do not establish release
readiness for these native lifecycle tests. Consumer Unix setup and actual signed
release execution remain separate work until their acceptance evidence exists.
