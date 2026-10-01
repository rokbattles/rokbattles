#!/usr/bin/env python3
"""Prepare Unix capture release assets; never install, start services or capture."""
from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import zipfile

import stage_package

IDENTIFIERS = {
    "rokbattles-capture-helper": "com.rokbattles.capture-helper",
    "rokbattles-desktop-agent": "com.rokbattles.desktop-agent",
}
FORBIDDEN = {
    "com.apple.security.get-task-allow",
    "com.apple.security.cs.disable-library-validation",
    "com.apple.security.cs.allow-dyld-environment-variables",
    "com.apple.security.cs.allow-unsigned-executable-memory",
    "com.apple.security.cs.allow-jit",
    "com.apple.security.cs.disable-executable-page-protection",
}


PIN_MARKER = b"EMBEDDED_RELEASE = None  # @ROKBATTLES_CAPTURE_RELEASE@"


class ReleaseError(Exception):
    pass


def execute(label: str, arguments: list[str], timeout: int = 120) -> bytes:
    """Never include argv in errors: notarization uses existing release secrets."""
    try:
        result = subprocess.run(arguments, capture_output=True, check=False, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        raise ReleaseError(f"{label} did not complete") from None
    if result.returncode != 0:
        raise ReleaseError(f"{label} failed; no release assets were approved")
    return result.stdout


def required_environment(environment: dict[str, str], name: str) -> str:
    value = environment.get(name, "")
    if not value:
        raise ReleaseError(f"Existing release setting {name} is required")
    return value


def sign_and_notarize(binaries: list[Path], directory: Path, environment: dict[str, str], runner=execute) -> dict:
    team = required_environment(environment, "APPLE_TEAM_ID")
    if not re.fullmatch(r"[A-Z0-9]{10}", team):
        raise ReleaseError("Existing Apple team identifier is invalid")
    identity = required_environment(environment, "APPLE_SIGNING_IDENTITY")
    apple_id = required_environment(environment, "APPLE_ID")
    password = required_environment(environment, "APPLE_PASSWORD")
    for binary in binaries:
        identifier = IDENTIFIERS[binary.name]
        requirement = f'=anchor apple generic and identifier "{identifier}" and certificate leaf[subject.OU] = "{team}"'
        runner("Hardened code signing", ["/usr/bin/codesign", "--force", "--timestamp", "--options", "runtime", "--identifier", identifier, "--sign", identity, str(binary)])
        runner("Signature and team verification", ["/usr/bin/codesign", "--verify", "--strict", "--all-architectures", "--test-requirement", requirement, str(binary)])
        entitlements = runner("Entitlement inspection", ["/usr/bin/codesign", "--display", "--entitlements", ":-", str(binary)])
        try:
            values = plistlib.loads(entitlements) if entitlements.strip() else {}
        except (ValueError, plistlib.InvalidFileException):
            raise ReleaseError("Cannot validate signed companion entitlements") from None
        if not isinstance(values, dict) or any(key in values and values[key] is not False for key in FORBIDDEN):
            raise ReleaseError("Companion has a forbidden code-injection entitlement")
    submitted = directory / "notarization.zip"
    with zipfile.ZipFile(submitted, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for binary in binaries:
            archive.write(binary, binary.name)
    result = runner("Apple notarization", ["/usr/bin/xcrun", "notarytool", "submit", str(submitted), "--apple-id", apple_id, "--password", password, "--team-id", team, "--wait", "--output-format", "json"], 1800)
    try:
        receipt = json.loads(result)
    except (ValueError, UnicodeError):
        raise ReleaseError("Apple notarization returned an invalid receipt") from None
    if not isinstance(receipt, dict) or receipt.get("status") != "Accepted" or not isinstance(receipt.get("id"), str):
        raise ReleaseError("Apple has not accepted the capture companion payload")
    # No password, account, upload URL or full server log is published.
    return {"id": receipt["id"], "status": "Accepted", "teamIdentifier": team}


def render_installer(template: bytes, target: str, archive: Path, team: str | None) -> bytes:
    if template.count(PIN_MARKER) != 1:
        raise ReleaseError("Installer must contain exactly one build-time release pin marker")
    pins = {
        "schemaVersion": 1,
        "target": target,
        "archiveName": archive.name,
        "archiveSha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        "appleTeamId": team,
    }
    source = template.replace(PIN_MARKER, ("EMBEDDED_RELEASE = " + repr(pins)).encode())
    ast.parse(source)
    return source


def prepare(target: str, helper: Path, agent: Path, installer: Path, output: Path, environment: dict[str, str], runner=execute) -> list[Path]:
    if target not in stage_package.TARGETS:
        raise ReleaseError("Unsupported Unix capture target")
    installer_bytes = installer.read_bytes()
    if len(installer_bytes) > 1024 * 1024:
        raise ReleaseError("Unexpected installer size")
    try:
        ast.parse(installer_bytes)
    except (SyntaxError, ValueError):
        raise ReleaseError("Administrative installer is not valid Python source") from None
    output.mkdir(parents=True, exist_ok=False)
    work = output / "signed"
    assets = output / "assets"
    work.mkdir()
    assets.mkdir()
    binaries = []
    for source, name in [(helper, "rokbattles-capture-helper"), (agent, "rokbattles-desktop-agent")]:
        with source.open("rb") as stream:
            data = stream.read(stage_package.MAX_BINARY + 1)
        stage_package.check_binary(data, target)
        destination = work / name
        destination.write_bytes(data)
        destination.chmod(0o755)
        binaries.append(destination)
    receipt = None
    if stage_package.TARGETS[target][0] == "macos":
        receipt = sign_and_notarize(binaries, work, environment, runner)
    archive = assets / f"rokbattles-capture-{target}.tar.gz"
    stage_package.stage(binaries[0], binaries[1], target, archive)
    maintenance = assets / f"rokbattles-capture-maintain-{target}.py"
    maintenance.write_bytes(render_installer(installer_bytes, target, archive, receipt["teamIdentifier"] if receipt else None))
    files = [archive, maintenance]
    if receipt is not None:
        notarization = assets / f"rokbattles-capture-{target}.notarization.json"
        notarization.write_text(json.dumps(receipt, indent=2) + "\n")
        files.append(notarization)
    checksum = assets / f"rokbattles-capture-{target}.sha256"
    checksum.write_text("".join(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n" for path in files))
    return [*files, checksum]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=stage_package.TARGETS, required=True)
    parser.add_argument("--helper", type=Path, required=True)
    parser.add_argument("--agent", type=Path, required=True)
    parser.add_argument("--installer", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        files = prepare(arguments.target, arguments.helper, arguments.agent, arguments.installer, arguments.output, dict(os.environ))
    except (ReleaseError, OSError, ValueError) as error:
        parser.exit(2, f"Capture release staging failed: {error}\n")
    for path in files:
        print(path)


if __name__ == "__main__":
    main()
