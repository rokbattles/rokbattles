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

## Administrator maintenance CLI

`maintain.py` is the source template for the standalone release installer produced
by `stage_release.py`. The release builder binds exactly one immutable archive
SHA-256, archive name, native target and (on macOS) Apple team. The unexpanded source
refuses installation. There is no CLI/environment digest, team, target, destination,
service name, command, filter or native-library override. Verify and use the
installer distributed with the matching release archive, rather than editing its
embedded pins. macOS release artifacts require the existing Developer ID signing
and accepted notarization process; the CLI also verifies both native signatures,
exact signing identifiers/team, hardened runtime and forbidden entitlements before
changing an installed image.

The CLI requires a trusted system Python 3 interpreter and isolated mode (`-I`).
The installer script and every directory ancestor must already be root-owned and
not group/other-writable; the runtime rejects a mutable invocation path. An
administrator must review/authenticate and provision that exact release script
through a trusted package or protected setup step before root invocation. Embedded
pins authenticate the payload after the script starts; they cannot authenticate a
user-writable script before the privileged interpreter opens it. This is an
explicit administrator/developer tool, not a finished secure consumer installer.
A signed native package or protected authenticated bootstrap remains required for
that consumer flow.
On macOS, `/usr/bin/python3` normally requires Apple's Command Line Tools; those
tools are an explicit setup prerequisite, not something this installer downloads
or installs. A signed native maintainer remains an option for a general consumer
setup experience. The CLI never invokes sudo, requests credentials, installs on
application launch, accepts an agreement or changes host security policy.

### First administrator setup

These are manual administrator steps, not commands run by the app. Start from a
trusted administrator shell and authenticate the official release page and its
installer checksum before granting privileges. Obtain the matching target's
generated installer, archive and checksum listing from that release; a checksum
downloaded from an unauthenticated mirror is not an independent trust anchor.

1. Inspect every ancestor of the setup directory for root ownership, absence of
   symlinks and group/other write access. On macOS, also inspect ACLs for mutation
   grants. Stop on an unsafe or ambiguous component; do not recursively change
   ownership or permissions to make an existing directory pass.
2. For the first setup, create a new root-only directory at
   `/usr/libexec/rokbattles-setup` on Linux (root:root), or
   `/Library/PrivilegedHelperTools/rokbattles-setup` on macOS (root:wheel), using
   `mkdir -m 0700` from that trusted administrator shell. Creation must fail if
   the directory already exists. On a later setup, independently verify the
   existing directory and ancestors before reusing it. If `/usr/libexec` is
   absent, provision that root:root, mode-0755 ancestor explicitly first.
3. Copy the generated release installer into the protected directory as a new
   `install-release.py` file, owned by root and mode 0600. Use a fresh filename
   for a later release or first remove only a previously verified setup script;
   never follow an existing destination symlink. Keep the archive under its
   original release filename; the installer snapshots and checks its immutable
   contents against the embedded archive hash before privileged changes.
4. Hash the **protected copy**, after the copy has completed. On Linux use
   `/usr/bin/sha256sum /usr/libexec/rokbattles-setup/install-release.py`; on macOS
   use `/usr/bin/shasum -a 256
   /Library/PrivilegedHelperTools/rokbattles-setup/install-release.py`. Compare the
   complete digest with the installer entry authenticated in step 1. Stop on any
   mismatch. Checking only the mutable downloaded source before copying is
   insufficient. Recheck the protected copy's owner, mode and ancestors.
5. Select the intended existing local user's numeric UID with the trusted OS
   account tools. Invoke the protected copy using the trusted system interpreter
   in isolated mode, with the matching archive's absolute path and explicit
   confirmation. For example, on Linux:

   ```sh
   /usr/bin/python3 -I /usr/libexec/rokbattles-setup/install-release.py install \
     --package /absolute/download/path/matching-release.tar.gz --uid 1000 \
     --confirm-system-changes
   ```

   On macOS, replace the script path with
   `/Library/PrivilegedHelperTools/rokbattles-setup/install-release.py` and select
   the actual account UID (commonly 501, but never assume it). The script itself
   verifies the protected invocation and refuses automatic elevation. Add
   `--all-registered-users` only when explicitly accepting interruption of the
   existing shared installation.
6. For updates, repeat authentication, protected copying and post-copy hashing
   for the new release installer, then use `update --package ...` and the same
   confirmation flags. Run `repair` or `uninstall --uid ...` from the authenticated
   protected installer for the installed target. Restart the normal app as the
   selected unprivileged user after successful maintenance.

An administrator explicitly selects the local UID and passes
`--confirm-system-changes`. Supported operations are:

- `install --package <matching-release.tar.gz> --uid <UID>` installs or adds one user
- `update --package <matching-release.tar.gz>` updates the existing registered set
- `uninstall --uid <UID>` removes that user's service, retaining shared binaries
  until the final registered user is removed
- `repair` recovers a recorded interrupted transaction using verified root-owned
  rollback files; it does not adopt arbitrary or unregistered files/services

Each command also needs `--confirm-system-changes`. When more than one user is
affected, `--all-registered-users` explicitly confirms interruption of the shared
installation. Adding a user cannot silently upgrade existing users' binaries.
No command reads user SQLite, private app state or upload credentials. After
maintenance, restart the ordinary unprivileged app to resume its saved desired
worker setting; a service restart alone never grants capture consent.

The fixed installation contains a root-owned `.maintenance` directory with a
bounded registry, release receipt, transaction journal, exclusive lock and staged
old/new files. Files and directory ancestors are opened without following links;
root ownership, no group/other write, no setuid/setgid and macOS mutation ACLs are
verified. Atomic writes fsync the file and its containing directory. Archive
verification reads immutable bounded bytes, rejects duplicate/unexpected paths,
links, devices, sparse/PAX overrides and wrong architecture, and never calls a
general archive extraction API.

A journal records the complete old and intended user/file sets before staging.
Before shared image changes, the fixed root-owned `.capture-maintenance` marker
asks user agents to drain and exit, every registered helper is stopped, and native
process/mapping inspection must establish that the protected images are no longer
in use. Unknown service registrations, systemd drop-ins, an unexpected effective
service command, invisible process evidence or a busy image fail closed. No
arbitrary PID is killed. The marker/journal survive failure and reboot.

Rollback files remain until the new payload, registry and service readiness have
all succeeded. Repair verifies hashes before restoring any old file, removes files
recorded originally absent, and verifies restored bytes again. Unix helpers reject
startup while the marker exists, so commit clears it only after the full protected
binary/registry validation and immediately before starting the fixed services. A
restart failure re-establishes the block and preserves recovery state.

The maintenance tests use temporary fixture roots and an injected service backend;
they never run native binaries, service managers, sudo or live capture. Actual
Linux/macOS admin install, multi-user stop/drain, ACL behavior, native process
mapping visibility, signature/notarization assessment, restart, reboot, rollback
and uninstall remain separate native acceptance checks. A passing fixture suite
is not a production installation sign-off.
