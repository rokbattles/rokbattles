# Unix capture broker (draft implementation)

The separate privileged helper supports native Linux x64/ARM64 and macOS
Intel/Apple Silicon source code. The unprivileged desktop agent remains responsible
for opt-in settings, lifecycle observation, decoding, mailcache, SQLite, upload and
UI-independent work. The helper accepts only `--service --uid <canonical UID>`
from the administrator's installed service definition. IPC accepts only Start and
Stop; it cannot choose paths, commands, libraries, filters, interfaces or addresses.

An authenticated Unix connection is scoped to its kernel UID and pinned peer PID
plus process birth. Linux socket ownership requires a fresh host TCP row with the
same UID/inode, a matching process UID/starttime/network namespace and socket FD.
macOS uses a C bridge compiled against Apple's SDK for libproc PID/FD/socket
queries, including socket generation, process birth and shared-FD rejection.
Missing, ambiguous, stale or inaccessible evidence fails closed. No native bytes
leave the helper until a fresh outbound SYN, server SYN/ACK and client ACK bind a
TCP generation, and ownership is rechecked immediately before each IPC write.

Native pcap opens OS-enumerated local interfaces, one inbound server handle and
one outbound metadata handle each. Filters restrict TCP to server ports 3101/5222.
Outbound application bytes are rejected; only zero-payload controls are delivered.
Loss counters from both handles, queue overflow, topology changes and native
errors retire the source. Raw packet buffers use RAII wiping in IPC queues.
Client FIN retains an owned server tail; resets retire only after the exact reset
record is successfully written.

## Explicit setup contract

Installing or starting the helper requires explicit administrator authorization.
The app must show: the helper runs as root, reads selected local network traffic,
attributes game connections to this user, and sends only this user's bounded game
records to the local agent. Include an explicit Install/Enable and Cancel choice.
Never grant packet capabilities to the GUI, widen `/dev/bpf*`, ask for passwords in
chat, install on launch, use arbitrary library paths, or silently fall back to a
user-wide packet dump. A missing helper should leave mailcache available and show
that native capture needs setup. Stop/disconnect ends that connection's consent.

Pending packaging checkpoint: launchd and systemd definitions and an inert package
staging tool are being added in this draft. They will not run installation or
change permissions in tests. macOS launchd runtime-directory bootstrap remains to
be completed before the first installable package is usable.

## Validation and limits

Linux x64 synthetic tests and Clippy pass in the implementation container. Tests do
not open native pcap, install services, run sudo or change host security settings.
Apple SDK compilation and native synthetic tests still need the macOS Intel and
ARM64 CI runners; Linux ARM64 also needs its native runner. No live capture or
privileged install/uninstall/restart validation has been performed. This draft is
not a release sign-off. macOS signed/notarized helper distribution and systemd
package integration require the release process and explicit setup testing.

The host helper cannot attribute another network namespace or sandbox-restricted
process. A Flatpak UI needs a host-installed agent/helper and an approved local IPC
bridge; granting broad host filesystem or device access to Flatpak is not a
supported workaround. If ownership is hidden by procfs policy, a sandbox, hardened
process restrictions, or unavailable kernel APIs, capture remains unavailable.
Global source loss requires a fresh user-agent connection and handshake.

## Primary references

- [Linux TCP proc interface](https://www.kernel.org/doc/html/latest/networking/proc_net_tcp.html)
- [Linux proc filesystem](https://docs.kernel.org/filesystems/proc.html)
- [Apple libproc declarations](https://github.com/apple-oss-distributions/xnu/blob/main/libsyscall/wrappers/libproc/libproc.h)
- [Apple socket and process structures](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)
- [systemd execution settings](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html)
