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
record is successfully written. Helper and agent set and verify zero process-local
soft/hard core limits before workers start; Linux also disables dumpability for
piped core collectors. Service definitions enforce zero core limits too, so a
capture crash cannot turn volatile packet buffers into an ordinary core dump.

## Explicit setup contract

Installing or starting the helper requires explicit administrator authorization.
The app must show: the helper runs as root, reads selected local network traffic,
attributes game connections to this user, and sends only this user's bounded game
records to the local agent. Include an explicit Install/Enable and Cancel choice.
Never grant packet capabilities to the GUI, widen `/dev/bpf*`, ask for passwords in
chat, install on launch, use arbitrary library paths, or silently fall back to a
user-wide packet dump. A missing helper should leave mailcache available and show
that native capture needs setup. Stop/disconnect ends that connection's consent.

`setup-contract.json` supplies the explicit consent copy and setup/unavailable
states for the unprivileged app. The systemd template creates the root-owned
runtime directory; launchd uses the helper's fixed directory bootstrap under a
pinned, root-owned `/private/var/run`. The helper never repairs an existing
symlink, foreign owner or unsafe directory mode. A per-UID root-owned lock prevents
duplicate listeners and permits safe stale-socket recovery after a crash.

`stage_package.py` creates an inert tarball containing the native helper and unprivileged agent companion, the
systemd template or a UID-neutral launchd template, this guide, the setup contract
and a SHA-256 manifest. It validates 64-bit ELF/Mach-O target architecture; it does
not execute the input binary, grant privileges, extract files, install software,
accept an agreement or start a service. Example (a local staging operation):

```sh
python3 stage_package.py --binary /path/to/built/rokbattles-capture-helper \
  --agent /path/to/built/rokbattles-desktop-agent \
  --target aarch64-apple-darwin --output capture-helper.tar.gz
```

After explicit administrator approval, the release installer must verify the
signed/notarized distribution and manifest, confirm the selected local UID,
render the selected UID into the fixed launchd template (release payloads contain no CI-user UID),
install binaries and service definitions at their fixed destinations as root (directories
0755, executable0755, definitions0644), and register only that user's instance.
Linux uses `rokbattles-capture@<UID>.service`; macOS uses
`com.rokbattles.capture-helper.<UID>` in the system launchd domain. The UID is
checked again against the kernel peer for each connection. No installer execution
is included in these tests. Distribution signing/notarization and platform
installer UX integration remain release work.

Only the fixed protected companion executable may authenticate to the helper:
`/usr/libexec/rokbattles/rokbattles-desktop-agent` on Linux and
`/Library/Application Support/ROK Battles/rokbattles-desktop-agent` on macOS.
The GUI launches this unprivileged companion for native capture; its bundled
sidecar remains the mailcache fallback. Never grant the companion root privileges.
On macOS, sign helper and companion with the same Apple signing team, explicitly
set identifiers `com.rokbattles.capture-helper` and
`com.rokbattles.desktop-agent`, and use hardened runtime without debugger, DYLD
environment, disabled library-validation, unsigned-executable-memory or JIT
entitlements. A filename alone is not a signing identifier. Runtime authentication
validates the helper anchor, installed and running agent requirements, signatures
and code hashes. Unsigned/ad-hoc debug builds cannot receive native capture.
Linux pins an opened `/proc/<pid>/exe` against the fixed root-owned image identity,
rejects deleted/memfd paths and nonzero TracerPid, and rechecks birth/UID. The agent
must set PR_SET_DUMPABLE=0 before connecting and the launcher must sanitize loader
environment overrides. Linux same-UID process-injection guarantees depend on OS
policy; this is not equivalent to macOS code-signing validation.

Disable/uninstall must first stop and await the agent and that service instance before removing
its definition or executable. Keep the shared binary and runtime directory while
another UID instance remains installed. The ordinary unprivileged Tauri updater cannot overwrite either protected binary.
Updates require the same stop/drain
sequence, verified new artifact and explicit restart. A service restart alone
never resumes capture; the unprivileged agent must authenticate and send Start
again.

## Validation and limits

The six native Capture Foundation jobs passed for broker checkpoint `080cc4da`,
including macOS Intel/ARM64 SDK compilation and Linux x64/ARM64 synthetic tests.
Package-staging fixtures and focused Clippy also pass. Tests do not open native
pcap, install services, run sudo or change host security settings. New follow-up
commits require their own exact-head CI checks. No live capture or privileged
install/uninstall/restart validation has been performed. This draft is
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
