#!/usr/bin/env python3
"""Build an inert administrator-reviewable tarball. Never install or run anything."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path
import plistlib
import struct
import tarfile

ROOT = Path(__file__).resolve().parent
TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux", 62),
    "aarch64-unknown-linux-gnu": ("linux", 183),
    "x86_64-apple-darwin": ("macos", 0x01000007),
    "aarch64-apple-darwin": ("macos", 0x0100000C),
}
MAX_BINARY = 128 * 1024 * 1024


def check_binary(data: bytes, target: str) -> None:
    platform, machine = TARGETS[target]
    if not 64 <= len(data) <= MAX_BINARY:
        raise ValueError("unexpected helper binary size")
    if platform == "linux":
        if data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", data, 18)[0] != machine:
            raise ValueError("helper must be a 64-bit executable for the selected target")
        if struct.unpack_from("<H", data, 16)[0] not in (2, 3):
            raise ValueError("helper must be an ELF executable or PIE")
    elif (data[:4] != b"\xcf\xfa\xed\xfe"
          or struct.unpack_from("<I", data, 4)[0] != machine
          or struct.unpack_from("<I", data, 12)[0] != 2):
        raise ValueError("helper must be a thin Mach-O executable for the selected target")


def payloads(binary: bytes, agent_binary: bytes, target: str) -> dict[str, tuple[bytes, int]]:
    check_binary(binary, target)
    check_binary(agent_binary, target)
    platform, _ = TARGETS[target]
    files: dict[str, tuple[bytes, int]] = {}
    if platform == "linux":
        helper = "usr/libexec/rokbattles/rokbattles-capture-helper"
        agent = "usr/libexec/rokbattles/rokbattles-desktop-agent"
        files["usr/lib/systemd/system/rokbattles-capture@.service"] = (
            (ROOT / "rokbattles-capture@.service").read_bytes(), 0o644
        )
    else:
        helper = "Library/PrivilegedHelperTools/com.rokbattles.capture-helper"
        agent = "Library/Application Support/ROK Battles/rokbattles-desktop-agent"
        source = (ROOT / "com.rokbattles.capture-helper.plist.in").read_text()
        document = plistlib.loads(source.encode())
        if document["ProgramArguments"] != [f"/{helper}", "--service", "--uid", "@UID@"]:
            raise ValueError("unexpected service command")
        files["share/rokbattles-capture/com.rokbattles.capture-helper.plist.in"] = (
            plistlib.dumps(document, sort_keys=False), 0o644
        )
    files[helper] = (binary, 0o755)
    files[agent] = (agent_binary, 0o755)
    files["share/rokbattles-capture/setup-contract.json"] = ((ROOT / "setup-contract.json").read_bytes(), 0o644)
    files["share/rokbattles-capture/README.md"] = ((ROOT / "README.md").read_bytes(), 0o644)
    manifest = {
        "schemaVersion": 1,
        "target": target,
        "requiresInstallerUserSelection": True,
        "installPerformed": False,
        "administratorConsentRequired": True,
        "signingAndNativeValidationRequired": True,
        "files": [
            {"path": path, "mode": oct(mode), "sha256": hashlib.sha256(data).hexdigest()}
            for path, (data, mode) in sorted(files.items())
        ],
    }
    files["manifest.json"] = ((json.dumps(manifest, indent=2) + "\n").encode(), 0o644)
    return files


def stage(binary_path: Path, agent_path: Path, target: str, output: Path) -> None:
    with binary_path.open("rb") as source:
        binary = source.read(MAX_BINARY + 1)
    with agent_path.open("rb") as source:
        agent_binary = source.read(MAX_BINARY + 1)
    files = payloads(binary, agent_binary, target)
    # Exclusive creation prevents following or replacing a supplied output symlink.
    with output.open("xb") as destination:
        with tarfile.open(fileobj=destination, mode="w:gz", format=tarfile.PAX_FORMAT) as archive:
            for path, (data, mode) in sorted(files.items()):
                info = tarfile.TarInfo(path)
                info.size = len(data)
                info.mode = mode
                info.uid = info.gid = 0
                info.uname = "root"
                info.gname = "root" if TARGETS[target][0] == "linux" else "wheel"
                info.mtime = 0
                archive.addfile(info, io.BytesIO(data))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--agent", type=Path, required=True, help="matching protected unprivileged agent companion")
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new tar.gz file; never installed")
    arguments = parser.parse_args()
    try:
        stage(arguments.binary, arguments.agent, arguments.target, arguments.output)
    except (ValueError, OSError) as error:
        parser.exit(2, f"Cannot stage helper package: {error}\n")
    print(f"Staged {arguments.output}; no installation or service action performed")


if __name__ == "__main__":
    main()
