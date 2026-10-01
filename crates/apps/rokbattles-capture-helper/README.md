# Capture-only service

This source checkpoint implements the Windows SCM host and authenticated local
transport. It is not an installer and does not enable live capture in tests.

The helper has no HTTP client, SQLite, mail decoder, updater, token, URL, arbitrary
filter, interface or path input. It runs only with `--service`. Each active WTS
logon gets one local named pipe; listing sessions never opens a capture handle.
Only an authenticated installed `rokbattles-desktop-agent.exe` may send `Start`.
`Stop`, disconnect, logout, malformed IPC, queue overflow and service stop end
consent and stop that capture. A reconnect must authenticate and start again.

## Windows trust boundary

- Fixed layout: Program Files/ROK Battles/{rokbattles-capture-helper.exe,
  rokbattles-desktop-agent.exe}; WinDivert files live below
  capture/windivert/x86_64-pc-windows-msvc
- Each path component must have an administrative owner and no non-administrative
  write ACE. Reparse points, unexpected final paths, unsupported ACL forms and
  ambiguous native buffers fail closed. Open handles deny write/delete sharing
- Named pipes use a protected SYSTEM + exact logon-SID DACL, first-instance and
  remote-client rejection. The logon ACE excludes create-pipe-instance rights.
  Impersonation verifies TokenUser and TokenLogonSid, then pins the originating
  process. Clients verify the SCM service PID, SYSTEM token and protected image
- The official unsigned WinDivert DLL is authenticated by compiled size/SHA-256
  pins. The driver has independent pins and exact embedded signature index 1
  DRIVER_ACTION_VERIFY with chain revocation and cache-only URL retrieval.
  A missing/expired trust cache fails closed; runtime never performs network trust
  retrieval. Explicit maintenance must establish the required trusted installation
- WinDivert's SCM driver must already be running from the exact pinned file.
  Runtime does not install or start it. The protected global mutex
  `Global\ROKBattles.Capture.NativeOpen.v1` serializes trusted loading/open with
  explicit maintenance. Maintenance must hold the same mutex while validating,
  installing and starting the driver, and stop the helper before replacing files
- Capture remains SNIFF | RECV_ONLY | NO_INSTALL. No Send or injection symbol is
  loaded. A native read or slow client cannot block or change original game traffic

## Attribution and loss

For WinDivert, only an observed outbound zero-payload SYN can create a binding.
An exact TCP owner row pins a process object, creation identity, user and logon.
Both handshake ISNs must match before server payload is forwarded; midstream
packets are dropped. Every emitted packet/control rechecks the exact live tuple,
process creation identity and current user/logon. Absent, retired, ambiguous or
changed ownership invalidates flow state. The table work, allocations, binding
count, handshake idle interval and active idle interval are bounded.

IPC accepts at most 65,535 bytes per body, known v1 kinds and zero reserved bits.
Malformed/truncated frames are fatal. Partial-frame and write/flush deadlines are
five seconds; idle healthy sessions send keepalives every two seconds. The native
queue holds at most 64 records (under 4.2 MiB of packet bodies). Overflow aborts the
connection before any post-loss record. PacketBytes wipes owned packet buffers on
rejection, completion, error and cancellation. Borrowed pcap-owned native memory
cannot be safely overwritten by this application.

## Current verification and remaining work

Synthetic framing, credential rejection, handshake, cross-user tuple replacement,
resource limits and interruption tests run without native capture. Windows and
macOS cross-compilation checks FFI/type correctness, not installation behavior.
CI tests the mock suites on six native architectures without loading a driver.

A full Windows installation/upgrade/uninstall, standard-user/SYSTEM pipe exchange,
revocation-cache behavior, service crash/restart and live authorized capture still
need explicit native acceptance testing. No application signing credentials are
configured by this change. Windows Npcap/ARM64 socket-evidence integration is a
separate follow-up; this checkpoint reports ownership unavailable there. Npcap
is never bundled. Unix transports have peer-credential checks, but Unix privileged
service provisioning and native capture hosts are not implemented by this slice.
