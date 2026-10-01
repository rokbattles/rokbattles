# Desktop native dependency checks

`desktop-native-dependencies.yml` is separate from the synthetic Capture
Foundation workflow. It runs native x64 and ARM64 runners on Windows, macOS and
Ubuntu, builds the desktop frontend and Tauri application plus agent/helper, and
executes their synthetic tests. Windows also builds the maintenance program.
It does not package, publish, install or launch the application.

The workflow also preserves the Capture Foundation and WinDivert Vendor checks:
capture pipeline formatting, lint and synthetic tests; Linux NAT/ingress
regressions; packaging fixtures; Windows bootstrap and compile-only NSIS checks;
and pinned vendor integrity and signature fixtures. Both Windows architectures
run the helper's exact offline driver-policy acceptance test against immutable
signed files. That test reads signatures without loading either fixture. The
existing workflows remain until this combined coverage passes on the exact
replacement commit.

## Actual runtime coverage

| Platform | Real native runtime load / adapter symbols | Live capture |
| --- | --- | --- |
| Ubuntu 24.04 x64 / ARM64 | `libpcap0.8t64` from the image's signed Ubuntu repositories | Not run |
| macOS 15 Intel / ARM64 | Apple's exact `/usr/lib/libpcap.A.dylib` system image, including dyld-cache-only systems | Not run |
| Windows x64 | Official pinned WinDivert 2.2.2-A DLL after archive/member hashes and Windows driver-policy signature checks | Not run |
| Windows x64 / ARM64 Npcap | **Blocked in hosted CI**; manual preinstalled-runtime probe below | Not run |
| Windows ARM64 WinDivert | Unsupported by the pinned upstream release | Not run |

The ignored tests call only `Pcap::load` / `WinDivert::load` and drop the library.
This executes trusted library initializers and resolves the production adapter's
complete required symbol set. It never calls `pcap_init`, `pcap_create`,
`pcap_activate`, `WinDivertOpen`, interface enumeration, driver/service lifecycle
APIs or receive functions. CI selects one exact ignored test, never `--ignored`
by itself. Ordinary tests also verify that missing libraries are recoverable
errors. Passing load-only checks does not establish capture permissions, driver
activation, packet delivery, installer behavior or application UI startup.

The WinDivert DLL is authenticated by the committed archive and member hashes;
the official DLL is unsigned. Its companion upstream-signed driver must pass the
existing pinned nested-signature and `DRIVER_ACTION_VERIFY` checks before the DLL
probe. CI never starts that driver. No signing key or other secret is needed.

## Why Npcap is explicitly blocked

The [official installation guide](https://npcap.com/guide/npcap-users-guide.html)
states that silent installation (`/S`) is available only with Npcap OEM. The
[official licensing page](https://npcap.com/)
describes the free edition's installation cap and prohibition on redistribution.
A public hosted job cannot treat downloading the SDK/import libraries as runtime
installation, accept the interactive installer on the user's behalf, or assume
an OEM license. This workflow therefore does not download, install, extract,
cache, upload or bundle any Npcap installer, SDK, DLL or driver. Users install
Npcap separately. Windows compile/synthetic success is not Npcap runtime success;
the workflow summary explicitly says **NOT RUN / BLOCKED**.

For a separately authorized Windows x64 or ARM64 machine where an administrator
has already accepted the license and installed Npcap, use native PowerShell 7:

```powershell
./.github/native-capture/probe-installed-npcap.ps1
```

The script accepts no library path or installer URL. It requires native process
architecture, fixed `System32/Npcap/wpcap.dll` and `Packet.dll`, and valid Nmap
Software LLC publisher signatures. Missing or untrusted runtime fails explicitly;
there is no mock/SDK fallback. The exact ignored test then resolves all adapter
symbols without initializing pcap or opening capture. Do not use a persistent
self-hosted machine to execute untrusted pull-request code.

## Native probe commands

After preparing the trusted runtime on the matching platform:

```sh
cargo test --locked -p rokbattles-capture-adapters --test native_dependencies pcap_system_library_loads_without_capture -- --ignored --exact
```

On Windows x64, first stage and verify the approved WinDivert payload:

```powershell
python crates/apps/rokbattles-desktop/scripts/windivert_vendor.py stage --target x86_64-pc-windows-msvc
./crates/apps/rokbattles-desktop/scripts/verify_windivert.ps1
cargo test --locked -p rokbattles-capture-adapters --test native_dependencies windivert_verified_library_loads_without_capture -- --ignored --exact
```

The [WinDivert documentation](https://www.reqrypt.org/windivert-doc.html) explains
that driver installation can occur at `WinDivertOpen`; these probes never call
it. The six runner labels are documented in GitHub's
[hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
