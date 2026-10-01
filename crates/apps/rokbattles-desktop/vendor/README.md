# Windows x64 capture resources

The desktop Windows x64 release includes the **official hash-pinned WinDivert
DLL and upstream-signed kernel driver**, plus upstream license and source
notices. It never includes Npcap. WinDivert's official 2.2.2 archive has no native
Windows ARM64 driver, so the ARM64 release has no WinDivert resources. The
application must report that platform limitation rather than download a driver.

## Build and verification

From the repository root, using Python 3.12+:

```sh
python crates/apps/rokbattles-desktop/scripts/windivert_vendor.py stage --target x86_64-pc-windows-msvc
python crates/apps/rokbattles-desktop/scripts/windivert_vendor.py verify --target x86_64-pc-windows-msvc
python -m unittest discover -s crates/apps/rokbattles-desktop/scripts -p 'test_windivert_*.py' -v
```

Staging is a build-time operation. For an offline build, add
`--archive /path/to/WinDivert-2.2.2-A.zip` to `stage`; the same pins apply. There is
no URL/hash override. `verify` is offline and does not trust a generated manifest
as a source of hashes. Neither command loads code or installs a driver.

On Windows, require the installed Windows SDK and PowerShell 7, then run:

```powershell
./crates/apps/rokbattles-desktop/scripts/verify_windivert.ps1
```

This first checks the exact resource set, sizes and SHA-256 hashes. It requires
`WinDivert.dll` to have the upstream `NotSigned` Authenticode status. The DLL is
authenticated through the official archive hash and its independent member hash.
The pinned 2.2.2-A driver has a primary publisher signature plus a nested Microsoft
Windows Hardware Compatibility Publisher signature. `kernelSignatureIndex` pins
the latter (index 1). Current Windows SDKs reject SignTool's `/kp` and `/ds`
options together, so verification has two mandatory steps: SignTool verifies
the selected signature's Authenticode chain and timestamp (`/pa /ds 1 /tw`), and
`windivert_signature.py` requires Windows `WinVerifyTrust` with
`DRIVER_ACTION_VERIFY` and `WSS_VERIFY_SPECIFIC` for that same index. Only a zero
status and the exact returned signature index are accepted. The driver policy
is Microsoft's WHQL/driver trust provider, separate from generic Authenticode.
Chain revocation checking is enabled; no trust errors are ignored.

The original file is held open read-only, denying writes and deletion during
driver-policy verification. No signature is extracted, replaced or re-signed,
and no trust stores or security settings change. Windows CI checks the pinned
signature passes and rejects the primary publisher signature, an absent index,
the unsigned DLL and a tampered disposable test copy. Missing tools, a changed
DLL state, warning exit codes and invalid signatures fail closed.
A valid signature does not guarantee a particular user's Windows
security policy or antivirus allows that driver; real installation/capture
smoke testing is separate and is not performed by these scripts or CI.

Only after those checks, build Windows x64 with this explicit overlay:

```sh
pnpm -F @rokbattles/desktop-client tauri build --target x86_64-pc-windows-msvc --bundles nsis --config tauri.windows-x64.conf.json
```

The release workflow performs both integrity and signature checks before that
build. Do not use this overlay for another architecture. It maps eight explicit
resource files to `capture/windivert/x86_64-pc-windows-msvc/` under the installed
application's resource directory, with no directory glob. The NSIS overlay uses
`perMachine` installation, defaulting to protected Program Files. The runtime
must independently require a trusted installation directory before privileged
loading. Vendor staging itself never registers or activates the driver. The
separate explicit NSIS maintenance flow performs protected helper/driver setup;
capture still requires the user's consent in the desktop app. See the
[desktop capture guide](../../../../docs/desktop-capture.md).

## Provenance and fail-closed staging

`windivert.lock.json` pins the official HTTPS release URL, archive byte size and
SHA-256, exact member paths, individual sizes and SHA-256, version and source
commit. Staging validates the complete ZIP directory, refuses duplicate or
case-colliding paths, traversal/absolute paths, encrypted entries and links or
special files. Download, archive expansion and per-member reads are bounded.
Only the five selected files are read and written; no archive extraction API or
upstream executable is used.

Generated output is ignored by Git. Its directory and ancestors must not be
symlinks or Windows junctions. Each file is atomically replaced, and the exact
manifest is written last. Restaging replaces expected files and removes only
the explicitly listed obsolete WinDivert files (`WinDivert32.sys`,
`WinDivert.lib`). Unexpected files, directories or links abort without deleting
them. This deliberately refuses to silently clean an untrusted or mixed-use
directory. The build workspace must not be writable by untrusted concurrent
processes. Integrity verification rejects missing, changed or additional files.

## License and updating

`LICENSE`, `README` and `VERSION` are copied unchanged from the pinned release.
`NOTICE.txt` identifies the library, its licensing and the unsigned-DLL/signed-
driver distinction. `SOURCE.txt` supplies the corresponding-source location at
the exact upstream commit, including build scripts. Distributors are responsible
for maintaining equivalent source access required by the LGPL. Keep these
notices with every redistributed binary; do not replace upstream license text
with the application's license.

To update, independently inspect a new **official** release, its source commit,
license and supported architectures, calculate archive/member pins, and review
the changes. Do not disable checks to accommodate new signatures; review the
new artifact and preserve kernel-policy validation. Confirm both vendor CI
jobs, then do separately authorized native installation tests before shipping.
Do not use Npcap or substitute community driver builds.

References:
- [Official WinDivert 2.2.2 release](https://github.com/basil00/WinDivert/releases/tag/v2.2.2)
- [Upstream installation/signing documentation](https://reqrypt.org/windivert-doc.html)
- [Windows trust providers and driver verification](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/nf-wintrust-winverifytrust)
- [Exact embedded-signature selection](https://learn.microsoft.com/en-us/windows/win32/api/wintrust/ns-wintrust-wintrust_signature_settings)
