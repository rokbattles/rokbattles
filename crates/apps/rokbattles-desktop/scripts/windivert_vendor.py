"""Build-time-only WinDivert staging; never loads a DLL or installs a driver."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import stat
import sys
import tempfile
import urllib.parse
import urllib.request
import zipfile

DESKTOP = Path(__file__).resolve().parents[1]
LOCK_PATH = DESKTOP / "vendor" / "windivert.lock.json"
TARGET = "x86_64-pc-windows-msvc"
OUTPUT = DESKTOP / "vendor" / "windivert" / TARGET
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024
MAX_EXPANDED_BYTES = 8 * 1024 * 1024
MAX_ENTRIES = 128
NOTICES = {"NOTICE.txt": "NOTICE.windivert.txt", "SOURCE.txt": "SOURCE.windivert.txt"}
# Only these obsolete files may be removed; never trust a previous manifest's paths.
STALE_NAMES = {"WinDivert32.sys", "WinDivert.lib"}


class VerificationError(ValueError):
    pass


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_bounded(stream, limit):
    data = bytearray()
    while chunk := stream.read(min(64 * 1024, limit + 1 - len(data))):
        data.extend(chunk)
        if len(data) > limit:
            raise VerificationError("Input exceeds its byte limit")
    return bytes(data)


class HttpsOnlyRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if urllib.parse.urlsplit(newurl).scheme != "https":
            raise VerificationError("Non-HTTPS download redirect rejected")
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def load_lock():
    lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
    if lock["target"] != TARGET:
        raise VerificationError("Unsupported WinDivert target in lock")
    return lock


def archive_bytes(lock, archive=None):
    if archive is not None:
        with archive.open("rb") as stream:
            return read_bounded(stream, MAX_ARCHIVE_BYTES)
    url = lock["archive"]["url"]
    if not url.startswith("https://github.com/basil00/WinDivert/releases/download/"):
        raise VerificationError("Only the official upstream release URL is allowed")
    opener = urllib.request.build_opener(HttpsOnlyRedirect())
    request = urllib.request.Request(url, headers={"User-Agent": "ROKBattles-vendor-build"})
    with opener.open(request, timeout=60) as stream:
        return read_bounded(stream, MAX_ARCHIVE_BYTES)


def safe_component(part):
    return bool(
        re.fullmatch(r"[A-Za-z0-9_.-]+", part)
        and part not in (".", "..")
        and not part.endswith(".")
        and not re.fullmatch(r"(?i)(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?", part)
    )


def validate_member(info):
    name = info.filename
    parts = name.rstrip("/").split("/")
    # No normalization, absolute paths, ADS, backslashes, NULs or Windows aliases.
    if info.orig_filename != name or "//" in name or not all(map(safe_component, parts)):
        raise VerificationError(f"Unsafe archive member: {name!r}")
    if info.flag_bits & (1 | 64):
        raise VerificationError(f"Encrypted archive member: {name}")
    kind = stat.S_IFMT(info.external_attr >> 16)
    if kind not in (0, stat.S_IFREG, stat.S_IFDIR):
        raise VerificationError(f"Non-regular archive member: {name}")
    if (kind == stat.S_IFDIR) != info.is_dir() and kind != 0:
        raise VerificationError(f"Inconsistent archive member type: {name}")
    if info.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED):
        raise VerificationError(f"Unsupported ZIP compression: {name}")


def unpack_verified(data, lock):
    pin = lock["archive"]
    if len(data) > MAX_ARCHIVE_BYTES or len(data) != pin["size"] or digest(data) != pin["sha256"]:
        raise VerificationError("Archive size or SHA-256 does not match the committed pin")
    payload = {}
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        infos = archive.infolist()
        if len(infos) > MAX_ENTRIES or sum(info.file_size for info in infos) > MAX_EXPANDED_BYTES:
            raise VerificationError("Archive expands beyond its limits")
        seen = set()
        for info in infos:
            validate_member(info)
            key = info.filename.rstrip("/").casefold()
            if key in seen:
                raise VerificationError("Duplicate or case-colliding archive members")
            seen.add(key)
        for output, member in lock["members"].items():
            if not safe_component(output):
                raise VerificationError("Unsafe output name in lock")
            try:
                info = archive.getinfo(member["path"])
            except KeyError as error:
                raise VerificationError(f"Missing pinned member: {member['path']}") from error
            if info.is_dir() or info.file_size != member["size"]:
                raise VerificationError(f"Incorrect member size: {info.filename}")
            with archive.open(info) as stream:
                content = read_bounded(stream, member["size"])
            if len(content) != member["size"] or digest(content) != member["sha256"]:
                raise VerificationError(f"Incorrect member SHA-256: {info.filename}")
            payload[output] = content
    if payload["VERSION"] != (lock["version"] + "\n").encode("ascii"):
        raise VerificationError("Unexpected upstream version")
    return payload


def is_link(path):
    return path.is_symlink() or (hasattr(path, "is_junction") and path.is_junction())


def safe_directory(root, create=False):
    root = root.absolute()
    for path in [*reversed(root.parents), root]:
        if is_link(path):
            raise VerificationError(f"Symlink or junction in staging path: {path}")
        if path.exists():
            if not path.is_dir():
                raise VerificationError(f"Non-directory staging path: {path}")
        elif create:
            path.mkdir()
        else:
            raise VerificationError("Staged vendor directory is missing")
    # Windows temp paths can contain 8.3 aliases (RUNNER~1). Resolve only after
    # checking every supplied ancestor for links/junctions, then use the canonical
    # spelling for all containment checks. Same-file identity is required.
    resolved = root.resolve(strict=True)
    if not root.samefile(resolved):
        raise VerificationError("Staging path escapes its resolved location")
    return resolved


def check_directory(root, names):
    for path in root.iterdir():
        if path.name not in names or is_link(path) or not path.is_file():
            raise VerificationError(f"Unexpected file or link in staging directory: {path.name}")
        if path.resolve().parent != root:
            raise VerificationError("Staged file escapes the staging directory")


def notices():
    return {name: (DESKTOP / "vendor" / source).read_bytes() for name, source in NOTICES.items()}


def expected_manifest(lock, extra):
    files = {
        name: {"size": member["size"], "sha256": member["sha256"]}
        for name, member in lock["members"].items()
    }
    files.update({name: {"size": len(data), "sha256": digest(data)} for name, data in extra.items()})
    manifest = {
        "schema": 1,
        "version": lock["version"],
        "target": TARGET,
        "archive": lock["archive"],
        "source": lock["source"],
        "signaturePolicy": {
            "WinDivert.dll": "NotSigned; authenticated by pinned archive and member SHA-256",
            "WinDivert64.sys": "upstream-signed; require Windows SignTool kernel-policy verification",
        },
        "files": files,
    }
    return (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode("utf-8")


def atomic_write(root, name, data):
    # Never use archive paths for output. Fixed direct children only; no extractall.
    fd, temporary = tempfile.mkstemp(prefix=".windivert-", dir=root)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, root / name)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def stage(data, lock, root=OUTPUT):
    payload = unpack_verified(data, lock)
    extra = notices()
    payload.update(extra)
    root = safe_directory(root, create=True)
    allowed = set(payload) | {"manifest.json"}
    check_directory(root, allowed | STALE_NAMES)
    # Invalidate the old manifest first; a partial/failed stage can never verify.
    for name in {"manifest.json"} | STALE_NAMES:
        (root / name).unlink(missing_ok=True)
    for name, content in payload.items():
        atomic_write(root, name, content)
    atomic_write(root, "manifest.json", expected_manifest(lock, extra))
    verify(lock, root)


def verify(lock, root=OUTPUT):
    root = safe_directory(root)
    extra = notices()
    expected = expected_manifest(lock, extra)
    files = json.loads(expected)["files"]
    check_directory(root, set(files) | {"manifest.json"})
    with (root / "manifest.json").open("rb") as stream:
        if read_bounded(stream, len(expected)) != expected:
            raise VerificationError("Staged manifest does not match the committed pin")
    for name, member in files.items():
        with (root / name).open("rb") as stream:
            data = read_bounded(stream, member["size"])
        if len(data) != member["size"] or digest(data) != member["sha256"]:
            raise VerificationError(f"Staged file does not match the pin: {name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("stage", "verify"))
    parser.add_argument("--target", required=True)
    parser.add_argument("--archive", type=Path, help="Use an offline copy of the pinned official ZIP")
    args = parser.parse_args()
    if args.target != TARGET:
        parser.error("Official WinDivert is available only for x86_64-pc-windows-msvc; never bundle on ARM64")
    if args.command == "verify" and args.archive is not None:
        parser.error("verify never downloads or reads an archive")
    lock = load_lock()
    try:
        if args.command == "stage":
            stage(archive_bytes(lock, args.archive), lock)
        else:
            verify(lock)
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError) as error:
        print(f"WinDivert verification failed: {error}", file=sys.stderr)
        return 1
    print(f"Verified WinDivert {lock['version']} resources for {TARGET}; no code loaded or driver installed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
