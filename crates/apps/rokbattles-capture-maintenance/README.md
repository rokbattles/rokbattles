# Windows capture maintenance (draft; native acceptance not run)

This executable is part of the **elevated per-machine NSIS installer**, not the
background agent or SYSTEM capture service. It accepts only `session`, `prepared`,
`commit`, `uninstall` and the read-only `validate-installer-lock`. It has no URL, filter, process, service-name or path
arguments. It never opens a user database, uploads packets, terminates an
arbitrary process, installs Npcap or downloads a driver.

The supported fixed layout is the OS `ProgramFiles` known folder plus `ROK Battles`:

- `rokbattles-desktop.exe`: normal user interface
- `rokbattles-desktop-agent.exe`: normal user worker
- `rokbattles-capture-helper.exe`: SYSTEM capture-only SCM service
- `.maintenance/rokbattles-capture-maintenance.exe`: elevated installer component
- `capture/windivert/x86_64-pc-windows-msvc/`: pinned official x64 vendor payload

Windows ARM64 installs the helper without WinDivert files or SCM dependency. Npcap
is a separate external prerequisite; absence of a native capture backend does not
justify bundling/installing it or elevating the mailcache worker.

## Installer lifetime and updater handoff

Tauri's Windows updater exits the UI after spawning the NSIS installer. A lock
owned by that UI cannot serialize the actual update. The UI therefore records its
separate maintenance-stop request, preserves desired enabled state, requests agent
exit and waits for the user agent lease. Elevated code never reads that SQLite
state. The independently running agent also checks the protected maintenance
marker before beginning work.

NSIS retains a separate protected InstallSession mutex handle through FINAL (or
installer process exit). Its owner/DACL is validated before acquisition; a second
installer is rejected throughout replacement and self-promotion. Acquisition order
is always InstallSession, then NativeOpen.

The old fixed maintenance executable starts a long-lived `session` before NSIS
copies any payload. It pins its own protected executable and ancestors, pins the
OS parent installer process handle/creation time, and requires elevated peers
whose OS parent and protected executable exactly match. A local-only admin/SYSTEM
pipe carries one-byte enum commands; callers cannot transmit identity, file paths,
service names or native options. Failed/unknown peers are disconnected. All waits
and session lifetime are bounded; installer death leaves capture unavailable.

The session holds `Global\ROKBattles.Capture.NativeOpen.v1` continuously through
quiescence, backup, NSIS replacement, integrity checks, driver registration/start,
and a passive receive-only `NO_INSTALL` open/close that reads no packets. The
mutex's existing owner/DACL is verified, so low-privilege name squatting is a
fail-closed denial of service. This lock only serializes ROK Battles components;
it does not control third-party WinDivert callers.

Before changing either service, maintenance verifies both existing names and
exact protected ImagePaths/types. A differently owned WinDivert service is a
conflict and is preserved. Helper configuration/delete rights are admin/SYSTEM
only; normal users receive query rights. Service command lines are quoted. On x64
both services auto-start with the helper depending on WinDivert, so runtime
`NO_INSTALL` still works after reboot. ARM64 has no WinDivert dependency.

Maintenance disables automatic starts, stops the helper before the driver, waits
for exact STOPPED state, and verifies the user agent has exited. It opens **all**
existing executable/DLL/SYS targets exclusively for read/write with only delete
sharing. Failure mutates no payload. It creates a bounded complete backup manifest
with exact names, present/absent entries, sizes, hashes and original file IDs,
flushes the copies and completion manifest, then renames the exclusively pinned
originals into protected retired directories. These handles stay pinned across
NSIS copying fresh paths. This prevents a late launch from the old paths; newly
copied agents remain blocked by the maintenance marker.

Commit verifies protected paths/ACLs/reparse policy, helper/agent/next-maintenance
package hashes and compiled WinDivert member hashes. The driver requires the
exact nested Microsoft signature (index 1), Windows DRIVER_ACTION_VERIFY and
revocation checking. Online chain retrieval is limited to maintenance; helper
runtime uses cached verification only. The session starts/probes the driver,
releases the mutex, starts the helper and writes the ready marker. It preserves
the maintenance block until the installer has atomically promoted the verified
next maintenance image and persisted its new hash. Only then does the fixed FINAL
hook remove the block. Normal unelevated UI relaunch resumes the user's desired
worker setting.

## Recovery and trust boundaries

`.capture-maintenance` presence or access uncertainty blocks worker start and all
native capture opens. Native capture additionally requires exact protected
`.capture-ready-v1` contents `ROKBattlesCaptureReady:1\n`. Missing readiness does
not itself prevent the ordinary mailcache worker.

An update error never clears the block. Where both services can be stopped, a
verified complete backup restores each previously present binary and removes
new files recorded as originally absent. Every restored file is hashed again.
Services stay disabled because unrelated UI resources may be partially updated;
rerun the installer to repair. Incomplete/tampered backup state fails closed.
A driver that cannot stop, an image still mapped, or a service marked for deletion
returns Windows exit code 3010. The installer aborts before replacing an active
image and asks for reboot/repair; it never schedules an active SYS overwrite.

Initial bootstrap uses only the explicit native system PowerShell with a fixed
compiled encoded command. It creates missing protected directories, refuses
untrusted existing owners/ACLs/reparse points, and authenticates the embedded
maintenance image using the SHA compiled into the installer. Upgrades start the
already installed image after checking protected `current.sha256`. A bounded
one/two-hash transition permits the same fixed old or new image across the atomic
self-swap; FINAL shrinks it to the new hash before unblocking. There is no temporary
or user-writable elevated executable fallback exists. App executables are
package-hash and protected-path authenticated. This does **not** claim application
Authenticode signing. The official WinDivert DLL is unsigned/hash-pinned; its SYS
has the upstream Microsoft kernel signature.

The PowerShell bootstrap commands are compressed solely to fit NSIS's bounded
command string, compiled into the installer, and receive no user interpolation.
No script is read from disk or fetched at runtime. A missing/untrusted maintenance
component requires full repair, rather than silently bootstrapping over an
existing active installation.

## Build and validation

1. Build/stage the agent with `scripts/stage_agent.py --target <Windows triple>`.
2. Run `scripts/stage_maintenance.py --target <Windows triple>`.
3. On x64, additionally stage and verify the official vendor files using the
   WinDivert packaging workflow and its `tauri.windows-x64.conf.json` overlay.
4. Include `tauri.capture-<Windows triple>.conf.json` and
   `tauri.agent.conf.json` when building the NSIS package.

Synthetic Rust lifecycle/rollback tests and packaging tests do not run the
maintenance binary, register/start services, load drivers or capture traffic.
Cross-check both Windows targets and run native CI tests/builds. Native NSIS
compilation, UAC, initial install, update cancellation/installer death, service
stop timeouts, repair/rollback, uninstall, reboot dependency ordering, SYSTEM trust
cache behavior and actual capture still require separately authorized Windows
acceptance testing. In particular, a valid installer-user trust check does not
prove the SYSTEM account's revocation cache is populated; runtime fails closed
if it is unavailable. Keep this PR draft until those checks pass.

Primary references: [Tauri NSIS hooks](https://v2.tauri.app/distribute/windows-installer/),
[service stop lifecycle](https://learn.microsoft.com/en-us/windows/win32/services/stopping-a-service),
[WinVerifyTrust](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust).
