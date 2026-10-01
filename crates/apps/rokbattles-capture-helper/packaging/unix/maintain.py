#!/usr/bin/python3 -I
"""Explicit administrator maintenance for fixed ROK capture installations.

No automatic elevation, downloads, user database access, shell commands or live
capture. The verified release builder embeds archive and signing pins before distribution;
neither its digest nor signing identity is a CLI or environment override.
"""
from __future__ import annotations

import argparse
import contextlib
import ctypes
import errno
import fcntl
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import plistlib
import pwd
import re
import stat
import struct
import subprocess
import sys
import tarfile
import time
import uuid

EMBEDDED_RELEASE = None  # @ROKBATTLES_CAPTURE_RELEASE@

MAX_ARCHIVE = 272 * 1024 * 1024
MAX_BINARY = 128 * 1024 * 1024
MAX_METADATA = 1024 * 1024
MAX_USERS = 256
TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux", 62),
    "aarch64-unknown-linux-gnu": ("linux", 183),
    "x86_64-apple-darwin": ("macos", 0x01000007),
    "aarch64-apple-darwin": ("macos", 0x0100000C),
}
DIGEST = re.compile(r"[a-f0-9]{64}\Z")
TEAM = re.compile(r"[A-Z0-9]{10}\Z")
SAFE_ENV = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "C", "LC_ALL": "C"}


class MaintenanceError(Exception):
    """Static diagnostics only; never include file content or command output."""


def require(condition, message):
    if not condition:
        raise MaintenanceError(message)


def uid_value(value):
    value = str(value)
    require(bool(re.fullmatch(r"[1-9][0-9]{0,9}", value)), "invalid local UID")
    uid = int(value)
    require(uid < 0xFFFFFFFF, "invalid local UID")
    return uid


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate metadata field")
        result[key] = value
    return result


def decode_json(data):
    require(len(data) <= MAX_METADATA, "metadata exceeds limit")
    try:
        return json.loads(data, object_pairs_hook=unique_object)
    except (UnicodeError, ValueError) as error:
        raise MaintenanceError("invalid metadata") from error


def sha(data):
    return hashlib.sha256(data).hexdigest()


def layout(target):
    require(isinstance(target, str) and target in TARGETS, "unsupported release target")
    if TARGETS[target][0] == "linux":
        root = "usr/libexec/rokbattles"
        helper = root + "/rokbattles-capture-helper"
        agent = root + "/rokbattles-desktop-agent"
        template = "usr/lib/systemd/system/rokbattles-capture@.service"
    else:
        root = "Library/Application Support/ROK Battles"
        helper = "Library/PrivilegedHelperTools/com.rokbattles.capture-helper"
        agent = root + "/rokbattles-desktop-agent"
        template = "share/rokbattles-capture/com.rokbattles.capture-helper.plist.in"
    return {"root": root, "helper": helper, "agent": agent, "template": template,
            "state": root + "/.maintenance", "marker": root + "/.capture-maintenance"}


def binary_matches(data, target):
    family, machine = TARGETS[target]
    require(64 <= len(data) <= MAX_BINARY, "invalid executable size")
    if family == "linux":
        require(data[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", data, 18)[0] == machine
                and struct.unpack_from("<H", data, 16)[0] in (2, 3), "wrong ELF architecture")
    else:
        require(data[:4] == b"\xcf\xfa\xed\xfe" and struct.unpack_from("<I", data, 4)[0] == machine
                and struct.unpack_from("<I", data, 12)[0] == 2, "wrong Mach-O architecture")


class Package:
    def __init__(self, target, digest, files, team):
        self.target, self.digest, self.files, self.team = target, digest, files, team
        self.paths = layout(target)

    @classmethod
    def read(cls, archive, trusted):
        require(isinstance(trusted, dict) and set(trusted) == {"schemaVersion", "target", "archiveSha256", "appleTeamId", "archiveName"}
                and trusted["schemaVersion"] == 1, "invalid protected release authority")
        target = trusted["target"]
        paths = layout(target)
        require(isinstance(trusted["archiveName"], str) and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", trusted["archiveName"]), "invalid embedded archive name")
        require(Path(archive).name == trusted["archiveName"], "archive name does not match this release installer")
        require(isinstance(trusted["archiveSha256"], str) and DIGEST.fullmatch(trusted["archiveSha256"]), "invalid protected release digest")
        team = trusted["appleTeamId"]
        require((TARGETS[target][0] == "linux" and team is None)
                or (isinstance(team, str) and TEAM.fullmatch(team)), "missing protected Apple signing identity")
        descriptor = os.open(archive, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC)
        with os.fdopen(descriptor, "rb") as source:
            info = os.fstat(source.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_size <= MAX_ARCHIVE, "invalid package file")
            # Immutable bytes are hashed and then parsed, avoiding a writable-input
            # hash/extract race. No archived executable is run.
            content = source.read(MAX_ARCHIVE + 1)
        require(len(content) <= MAX_ARCHIVE and sha(content) == trusted["archiveSha256"], "release archive authentication failed")
        allowed = {paths["helper"]: 0o755, paths["agent"]: 0o755, paths["template"]: 0o644,
                   "share/rokbattles-capture/README.md": 0o644,
                   "share/rokbattles-capture/setup-contract.json": 0o644, "manifest.json": 0o644}
        files = {}
        try:
            with tarfile.open(fileobj=io.BytesIO(content), mode="r:gz") as source:
                for member in source:
                    require(member.name in allowed and member.name not in files, "unexpected or duplicate package path")
                    require(member.isreg() and not member.issparse() and not member.pax_headers
                            and member.uid == member.gid == 0 and member.mode == allowed[member.name], "unsafe archive member")
                    limit = MAX_BINARY if member.mode == 0o755 else MAX_METADATA
                    require(0 <= member.size <= limit, "archive member exceeds limit")
                    stream = source.extractfile(member)
                    require(stream is not None, "missing archive member")
                    data = stream.read(limit + 1)
                    require(len(data) == member.size, "truncated archive member")
                    files[member.name] = data
        except (tarfile.TarError, EOFError, OSError) as error:
            raise MaintenanceError("invalid release archive") from error
        require(set(files) == set(allowed), "incomplete package inventory")
        manifest = decode_json(files.pop("manifest.json"))
        require(isinstance(manifest, dict) and manifest.get("schemaVersion") == 1
                and manifest.get("target") == target and manifest.get("requiresInstallerUserSelection") is True
                and manifest.get("installPerformed") is False and manifest.get("administratorConsentRequired") is True
                and manifest.get("signingAndNativeValidationRequired") is True and "uid" not in manifest, "package must select its user only during installation")
        entries = manifest.get("files")
        require(isinstance(entries, list) and len(entries) == len(files), "invalid package manifest")
        seen = set()
        for entry in entries:
            require(isinstance(entry, dict) and set(entry) == {"path", "mode", "sha256"}, "invalid manifest entry")
            path = entry["path"]
            require(isinstance(path, str) and path in files and path not in seen and entry["mode"] == oct(allowed[path])
                    and entry["sha256"] == sha(files[path]), "package member integrity failed")
            seen.add(path)
        binary_matches(files[paths["helper"]], target)
        binary_matches(files[paths["agent"]], target)
        return cls(target, trusted["archiveSha256"], files, team)


class FileSystem:
    """All destination names are compiled paths or validated journal inventory.

    Tests inject a temporary root and their own UID; CLI always uses '/' and UID0.
    """
    def __init__(self, root=Path("/"), owner=0):
        self.root, self.owner = Path(root), owner

    def absolute(self, relative):
        path = PurePosixPath(relative)
        require(not path.is_absolute() and all(part not in ("", ".", "..") for part in path.parts), "invalid fixed path")
        return self.root.joinpath(*path.parts)

    def check(self, descriptor, directory=False):
        info = os.fstat(descriptor)
        require(info.st_uid == self.owner and info.st_mode & 0o6022 == 0
                and (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)), "unprotected installation object")
        if sys.platform == "darwin":
            verify_macos_acl(descriptor)
        return info

    @contextlib.contextmanager
    def parent(self, relative, create=False):
        path = self.absolute(relative)
        pieces = path.relative_to(self.root).parts
        with contextlib.ExitStack() as stack:
            descriptor = os.open(self.root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
            stack.callback(os.close, descriptor)
            self.check(descriptor, True)
            for part in pieces[:-1]:
                try:
                    child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=descriptor)
                except FileNotFoundError:
                    if not create:
                        raise
                    mode = 0o700 if part == ".maintenance" or part.startswith("txn-") else 0o755
                    os.mkdir(part, mode, dir_fd=descriptor)
                    os.fsync(descriptor)
                    child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=descriptor)
                    os.fchmod(child, mode)
                    os.fsync(child)
                stack.callback(os.close, child)
                self.check(child, True)
                descriptor = child
            yield descriptor, pieces[-1]

    def read(self, relative, limit=MAX_METADATA, missing=None):
        try:
            with self.parent(relative) as (parent, name):
                descriptor = os.open(name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
                with os.fdopen(descriptor, "rb") as source:
                    info = self.check(source.fileno())
                    require(info.st_size <= limit, "protected file exceeds limit")
                    data = source.read(limit + 1)
                    require(len(data) <= limit, "protected file grew beyond limit")
                    return data
        except FileNotFoundError:
            return missing

    def write(self, relative, data, mode=0o644):
        require(len(data) <= MAX_BINARY and mode in (0o600, 0o644, 0o755), "invalid protected write")
        with self.parent(relative, True) as (parent, name):
            try:
                old = os.open(name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
            except FileNotFoundError:
                old = None
            if old is not None:
                try:
                    self.check(old)
                finally:
                    os.close(old)
            temporary = "." + name + ".stage-" + uuid.uuid4().hex
            descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=parent)
            try:
                with os.fdopen(descriptor, "wb") as output:
                    output.write(data)
                    output.flush()
                    os.fchmod(output.fileno(), mode)
                    os.fsync(output.fileno())
                os.replace(temporary, name, src_dir_fd=parent, dst_dir_fd=parent)
                os.fsync(parent)
            finally:
                try:
                    os.unlink(temporary, dir_fd=parent)
                except FileNotFoundError:
                    pass

    def delete(self, relative):
        try:
            with self.parent(relative) as (parent, name):
                try:
                    descriptor = os.open(name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
                except FileNotFoundError:
                    return
                try:
                    self.check(descriptor)
                finally:
                    os.close(descriptor)
                os.unlink(name, dir_fd=parent)
                os.fsync(parent)

        except FileNotFoundError:
            return

    def metadata(self, relative):
        with self.parent(relative) as (parent, name):
            descriptor = os.open(name, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
            try:
                return self.check(descriptor)
            finally:
                os.close(descriptor)

    @contextlib.contextmanager
    def lock(self, relative):
        with self.parent(relative, True) as (parent, name):
            descriptor = os.open(name, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=parent)
            try:
                self.check(descriptor)
                try:
                    fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError as error:
                    raise MaintenanceError("another maintenance operation is active") from error
                yield
            finally:
                os.close(descriptor)


def verify_macos_acl(descriptor):
    # Read-only Darwin filesec/ACL ABI, matching unix_trust_macos.c. The stat
    # output is opaque and generously aligned/sized; no native pointer is parsed.
    lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
    pointer = ctypes.c_void_p
    declarations = {
        "filesec_init": ([], pointer), "filesec_free": ([pointer], None),
        "fstatx_np": ([ctypes.c_int, pointer, pointer], ctypes.c_int),
        "filesec_query_property": ([pointer, ctypes.c_int, ctypes.POINTER(ctypes.c_int)], ctypes.c_int),
        "filesec_get_property": ([pointer, ctypes.c_int, pointer], ctypes.c_int),
        "acl_valid": ([pointer], ctypes.c_int), "acl_free": ([pointer], ctypes.c_int),
        "acl_get_entry": ([pointer, ctypes.c_int, ctypes.POINTER(pointer)], ctypes.c_int),
        "acl_get_tag_type": ([pointer, ctypes.POINTER(ctypes.c_int)], ctypes.c_int),
        "acl_get_permset_mask_np": ([pointer, ctypes.POINTER(ctypes.c_uint64)], ctypes.c_int),
    }
    for name, (arguments, result) in declarations.items():
        function = getattr(lib, name)
        function.argtypes, function.restype = arguments, result
    security = lib.filesec_init()
    require(security, "macOS ACL state unavailable")
    acl, present = pointer(), ctypes.c_int()
    try:
        opaque_stat = (ctypes.c_uint64 * 128)()
        require(lib.fstatx_np(descriptor, opaque_stat, security) == 0
                and lib.filesec_query_property(security, 5, ctypes.byref(present)) == 0, "macOS ACL query failed")
        if not present.value:
            return
        require(lib.filesec_get_property(security, 5, ctypes.byref(acl)) == 0 and acl.value
                and lib.acl_valid(acl) == 0, "invalid macOS ACL")
        safe = (1 << 1) | (1 << 3) | (1 << 7) | (1 << 9) | (1 << 11) | (1 << 20)
        for index in range(129):
            entry, tag, permissions = pointer(), ctypes.c_int(), ctypes.c_uint64()
            ctypes.set_errno(0)
            result = lib.acl_get_entry(acl, 0 if index == 0 else -1, ctypes.byref(entry))
            if result != 0:
                require(ctypes.get_errno() == errno.EINVAL, "macOS ACL enumeration failed")
                return
            require(index < 128 and lib.acl_get_tag_type(entry, ctypes.byref(tag)) == 0
                    and lib.acl_get_permset_mask_np(entry, ctypes.byref(permissions)) == 0, "macOS ACL entry unavailable")
            require(tag.value == 2 or (tag.value == 1 and permissions.value & ~safe == 0), "macOS ACL permits mutation")
        raise MaintenanceError("macOS ACL exceeds limit")
    finally:
        if acl.value:
            lib.acl_free(acl)
        lib.filesec_free(security)


def service_path(target, uid):
    uid = uid_value(uid)
    if TARGETS[target][0] == "linux":
        return "usr/lib/systemd/system/rokbattles-capture@.service"
    return f"Library/LaunchDaemons/com.rokbattles.capture-helper.{uid}.plist"


def definition(package, uid):
    uid = uid_value(uid)
    data = package.files[package.paths["template"]]
    if TARGETS[package.target][0] == "linux":
        text = data.decode("utf-8")
        directives = [line.strip() for line in text.splitlines() if line.strip() and not line.lstrip().startswith(("#", "["))]
        require(all("=" in line and "\\" not in line and "\x00" not in line for line in directives), "invalid service definition")
        for name in ("ExecStart", "User", "Group", "WorkingDirectory"):
            values = [line.split("=", 1)[1] for line in directives if line.startswith(name + "=")]
            expected = {"ExecStart": "/" + package.paths["helper"] + " --service --uid %i", "User": "root", "Group": "root", "WorkingDirectory": "/"}[name]
            require(values == [expected], "unexpected service privilege or command")
        require(not any(line.startswith("Exec") and not line.startswith("ExecStart=") for line in directives), "service has additional commands")
        require("NoNewPrivileges=yes" in directives and "Restart=on-failure" in directives and "LimitCORE=0" in directives, "missing service confinement")
        return data
    try:
        value = plistlib.loads(data)
    except (ValueError, plistlib.InvalidFileException) as error:
        raise MaintenanceError("invalid launchd template") from error
    require(value.get("Label") == "com.rokbattles.capture-helper.@UID@"
            and value.get("ProgramArguments") == ["/" + package.paths["helper"], "--service", "--uid", "@UID@"]
            and value.get("UserName") == "root" and value.get("GroupName") == "wheel"
            and value.get("WorkingDirectory") == "/"
            and value.get("EnvironmentVariables") == {"PATH": SAFE_ENV["PATH"], "LANG": "C"}, "unexpected launchd privilege or command")
    require(set(value) == {"Label", "ProgramArguments", "UserName", "GroupName", "WorkingDirectory", "EnvironmentVariables",
                           "Umask", "RunAtLoad", "KeepAlive", "ThrottleInterval", "ExitTimeOut", "ProcessType", "StandardOutPath", "StandardErrorPath", "SoftResourceLimits", "HardResourceLimits"}, "unexpected launchd fields")
    require(value["Umask"] == 63 and value["RunAtLoad"] is True and value["KeepAlive"] == {"SuccessfulExit": False}
            and value["ThrottleInterval"] == 5 and value["ExitTimeOut"] == 10 and value["ProcessType"] == "Background"
            and value["StandardOutPath"] == value["StandardErrorPath"] == "/dev/null"
            and value["SoftResourceLimits"] == value["HardResourceLimits"] == {"Core": 0}, "unsafe launchd execution settings")
    value["Label"] = f"com.rokbattles.capture-helper.{uid}"
    value["ProgramArguments"][-1] = str(uid)
    return plistlib.dumps(value, sort_keys=False)


class NativeServices:
    """Fixed system commands only; never run package executables during validation."""
    def __init__(self, fs, target):
        self.fs, self.target, self.paths = fs, target, layout(target)
        self.family = TARGETS[target][0]

    def command(self, arguments, accepted=(0,)):
        require(arguments[0] in ("/usr/bin/systemctl", "/bin/launchctl", "/usr/bin/codesign", "/usr/sbin/spctl", "/usr/sbin/lsof"), "unexpected maintenance command")
        completed = subprocess.run(arguments, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   timeout=20, env=SAFE_ENV, cwd="/", close_fds=True, check=False)
        require(len(completed.stdout) + len(completed.stderr) <= MAX_METADATA, "service response exceeds limit")
        require(completed.returncode in accepted, "native service/signature operation failed")
        return completed

    def name(self, uid):
        uid = uid_value(uid)
        return f"rokbattles-capture@{uid}.service" if self.family == "linux" else f"system/com.rokbattles.capture-helper.{uid}"

    def state(self, uid):
        name = self.name(uid)
        if self.family == "linux":
            result = self.command(["/usr/bin/systemctl", "show", name, "--no-pager", "--property=LoadState,ActiveState,SubState,MainPID,ExecStart,FragmentPath,DropInPaths"], (0, 1, 3, 4))
            fields = {}
            for line in result.stdout.decode("utf-8", "strict").splitlines():
                key, _, value = line.partition("=")
                require(key not in fields, "duplicate service state field")
                fields[key] = value
            if fields.get("LoadState") == "not-found":
                return "absent", 0
            require(fields.get("LoadState") == "loaded", "service load state unavailable")
            require(fields.get("FragmentPath") == "/" + service_path(self.target, uid)
                    and fields.get("DropInPaths") == "", "service overrides are not supported")
            command = fields.get("ExecStart", "")
            require(command.startswith("{ ") and command.endswith(" }") and command.count("{") == command.count("}") == 1, "ambiguous service command")
            pieces = [part.strip().partition("=") for part in command[2:-2].split(";")]
            values = unique_object((key, value) for key, separator, value in pieces if separator)
            require(set(values) == {"path", "argv[]", "ignore_errors", "start_time", "stop_time", "pid", "code", "status"}
                    and values["path"] == "/" + self.paths["helper"]
                    and values["argv[]"] == "/" + self.paths["helper"] + f" --service --uid {uid_value(uid)}"
                    and values["ignore_errors"] == "no", "service process has an unexpected command")
            if fields.get("ActiveState") == "active":
                return "running", int(fields.get("MainPID", "0"))
            if fields.get("ActiveState") in ("inactive", "failed"):
                return "stopped", 0
            return "pending", 0
        result = self.command(["/bin/launchctl", "print", name], tuple(range(256)))
        if result.returncode != 0:
            require(b"Could not find service" in result.stderr and not result.stdout, "launchd service state unavailable")
            return "absent", 0
        text = result.stdout.decode("utf-8", "strict")
        programs = [line.strip() for line in text.splitlines() if line.strip().startswith("program = ")]
        require(programs == ["program = /" + self.paths["helper"]], "launchd has an unexpected executable")
        arguments = re.findall(r"(?ms)^\s*arguments = \{\n(.*?)^\s*\}", text)
        require(len(arguments) == 1 and [line.strip() for line in arguments[0].splitlines() if line.strip()]
                == ["/" + self.paths["helper"], "--service", "--uid", str(uid_value(uid))], "launchd has unexpected arguments")
        match = re.search(r"(?m)^\s*pid = ([1-9][0-9]*)\s*$", text)
        return ("running", int(match.group(1))) if match else ("pending", 0)

    def stop(self, uid):
        state, _ = self.state(uid)
        if self.family == "linux":
            if state != "absent":
                self.command(["/usr/bin/systemctl", "disable", "--now", self.name(uid)])
        elif state != "absent":
            self.command(["/bin/launchctl", "bootout", self.name(uid)])
        deadline = time.monotonic() + 30
        while True:
            state, _ = self.state(uid)
            if state in ("stopped", "absent"):
                return
            require(time.monotonic() < deadline, "helper did not stop; maintenance remains blocked")
            time.sleep(0.1)

    def reload(self):
        if self.family == "linux":
            self.command(["/usr/bin/systemctl", "daemon-reload"])

    def start(self, uid):
        if self.family == "linux":
            state, _ = self.state(uid)
            require(state == "stopped", "effective service configuration is not ready to start")
            self.command(["/usr/bin/systemctl", "enable", "--now", self.name(uid)])
        else:
            self.command(["/bin/launchctl", "bootstrap", "system", "/" + service_path(self.target, uid)])
        deadline = time.monotonic() + 20
        while True:
            state, pid = self.state(uid)
            if state == "running" and pid > 0:
                require(self.process_image(pid) == self.fs.absolute(self.paths["helper"]), "unexpected running helper identity")
                return
            require(time.monotonic() < deadline, "helper readiness unavailable")
            time.sleep(0.1)

    def process_image(self, pid):
        if self.family == "linux":
            return Path(os.readlink(f"/proc/{pid}/exe"))
        lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
        lib.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
        lib.proc_pidpath.restype = ctypes.c_int
        output = ctypes.create_string_buffer(4096)
        count = lib.proc_pidpath(pid, output, len(output))
        require(0 < count < len(output), "process path unavailable")
        return Path(os.fsdecode(output.value))

    def wait_images_absent(self, identities):
        deadline = time.monotonic() + 30
        while self.images_in_use(identities):
            require(time.monotonic() < deadline, "protected process still running; close ROK Battles and retry repair")
            time.sleep(0.1)

    def images_in_use(self, identities):
        if not identities:
            return False
        if self.family == "macos":
            # lsof is the protected OS vnode/mapping inspector. It selects the
            # exact inode identities of these fixed files, including text mappings.
            paths = [str(self.fs.absolute(path)) for path in identities]
            result = self.command(["/usr/sbin/lsof", "-nP", "-F0pft", "--", *paths], (0, 1))
            require(not result.stderr, "macOS image mapping inventory unavailable")
            return bool(result.stdout)
        wanted = {(info[0], info[1]) for info in identities.values()}
        total = 0
        pids = [entry for entry in Path("/proc").iterdir() if entry.name.isdecimal()]
        require(len(pids) <= 65536, "process inventory exceeds limit")
        for process in pids:
            try:
                before = (process / "stat").read_bytes()
                birth = before.rsplit(b") ", 1)[1].split()[19]
                try:
                    image = (process / "exe").stat()
                    if (image.st_dev, image.st_ino) in wanted:
                        return True
                except FileNotFoundError:
                    pass
                with (process / "maps").open("rb") as source:
                    rows = source.read(8 * 1024 * 1024 + 1)
                total += len(rows)
                require(len(rows) <= 8 * 1024 * 1024 and total <= 64 * 1024 * 1024, "mapping inventory exceeds limit")
                for row in rows.splitlines():
                    fields = row.split(None, 5)
                    require(len(fields) >= 5, "invalid process mapping")
                    major, minor = (int(value, 16) for value in fields[3].split(b":"))
                    if (os.makedev(major, minor), int(fields[4])) in wanted:
                        return True
                after = (process / "stat").read_bytes().rsplit(b") ", 1)[1].split()[19]
                require(birth == after, "process inventory changed; retry maintenance")
            except (FileNotFoundError, ProcessLookupError):
                continue
            except (PermissionError, ValueError, IndexError) as error:
                raise MaintenanceError("cannot prove protected image quiescence") from error
        return False

    def validate_signatures(self, staged, team):
        if self.family != "macos":
            return
        require(isinstance(team, str) and TEAM.fullmatch(team), "missing trusted signing team")
        forbidden = {"com.apple.security.get-task-allow", "com.apple.security.cs.allow-dyld-environment-variables",
                     "com.apple.security.cs.disable-library-validation", "com.apple.security.cs.allow-unsigned-executable-memory",
                     "com.apple.security.cs.allow-jit", "com.apple.security.cs.disable-executable-page-protection"}
        for role, identifier in (("helper", "com.rokbattles.capture-helper"), ("agent", "com.rokbattles.desktop-agent")):
            path = str(staged[role])
            requirement = f'=anchor apple generic and identifier "{identifier}" and certificate leaf[subject.OU] = "{team}"'
            self.command(["/usr/bin/codesign", "--verify", "--strict", "--test-requirement", requirement, path])
            detail = self.command(["/usr/bin/codesign", "--display", "--verbose=4", path]).stderr.decode("utf-8", "strict")
            flags = re.search(r"(?m)^CodeDirectory .* flags=(0x[0-9a-fA-F]+)", detail)
            require(flags and int(flags.group(1), 16) & 0x10000 and not int(flags.group(1), 16) & 2, "hardened signed executable required")
            entitlements = self.command(["/usr/bin/codesign", "--display", "--entitlements", ":-", path]).stdout
            values = plistlib.loads(entitlements) if entitlements else {}
            require(isinstance(values, dict) and not any(values.get(key) for key in forbidden), "unsafe signing entitlement")
            assessment = self.command(["/usr/sbin/spctl", "--assess", "--type", "execute", "--verbose=4", path])
            require(b"source=Notarized Developer ID" in assessment.stderr, "notarized release approval required")

    def assert_inventory(self, registered):
        discovered = set()
        if self.family == "linux":
            for verb in ("list-units", "list-unit-files"):
                command = ["/usr/bin/systemctl", verb, "rokbattles-capture@*.service", "--no-legend", "--no-pager"]
                if verb == "list-units":
                    command.append("--all")
                output = self.command(command).stdout.decode("utf-8", "strict")
                for line in output.splitlines():
                    match = re.search(r"(?:^|\s)rokbattles-capture@([1-9][0-9]*)\.service(?:\s|$)", line)
                    if match:
                        discovered.add(uid_value(match.group(1)))
        else:
            directory = self.fs.absolute("Library/LaunchDaemons")
            for entry in os.scandir(directory):
                match = re.fullmatch(r"com\.rokbattles\.capture-helper\.([1-9][0-9]*)\.plist", entry.name)
                if match:
                    discovered.add(uid_value(match.group(1)))
        require(len(discovered) <= MAX_USERS and discovered <= set(registered), "unregistered capture service conflicts with maintenance")


class Installer:
    def __init__(self, fs, services, target, pins):
        self.fs, self.services, self.target, self.pins = fs, services, target, pins
        self.paths = layout(target)
        self.registry_path = self.paths["state"] + "/registry.json"
        self.journal_path = self.paths["state"] + "/journal.json"

    def empty(self):
        return {"schemaVersion": 1, "target": self.target, "uids": [], "files": {}, "release": None}

    def validate_registry(self, registry, check_files=False):
        require(isinstance(registry, dict) and set(registry) == {"schemaVersion", "target", "uids", "files", "release"}
                and registry["schemaVersion"] == 1 and registry["target"] == self.target, "invalid protected registry")
        uids = registry["uids"]
        require(isinstance(uids, list) and len(uids) <= MAX_USERS and all(type(uid) is int for uid in uids)
                and uids == sorted({uid_value(uid) for uid in uids}), "invalid registered user set")
        expected = {self.paths["helper"], self.paths["agent"]} if uids else set()
        expected.update(service_path(self.target, uid) for uid in uids)
        require(isinstance(registry["files"], dict) and set(registry["files"]) == expected, "unexpected installed file inventory")
        for path, record in registry["files"].items():
            mode = 0o755 if path in (self.paths["helper"], self.paths["agent"]) else 0o644
            require(isinstance(record, dict) and set(record) == {"sha256", "mode"} and record["mode"] == mode
                    and isinstance(record["sha256"], str) and DIGEST.fullmatch(record["sha256"]), "invalid installed file record")
            if check_files:
                data = self.fs.read(path, MAX_BINARY)
                require(data is not None and sha(data) == record["sha256"]
                        and stat.S_IMODE(self.fs.metadata(path).st_mode) == mode, "installed image or service changed")
        if uids:
            release = registry["release"]
            require(isinstance(release, dict) and release.get("target") == self.target
                    and isinstance(release.get("archiveSha256"), str) and DIGEST.fullmatch(release["archiveSha256"]), "missing protected release receipt")
        return registry

    def registry(self):
        content = self.fs.read(self.registry_path)
        registry = self.empty() if content is None else decode_json(content)
        return self.validate_registry(registry, True)

    def persist(self, path, value):
        self.fs.write(path, (json.dumps(value, sort_keys=True) + "\n").encode(), 0o600)

    def images(self):
        result = {}
        for key in ("helper", "agent"):
            path = self.paths[key]
            data = self.fs.read(path, MAX_BINARY)
            if data is not None:
                info = self.fs.metadata(path)
                result[path] = [info.st_dev, info.st_ino]
        return result

    def _journal(self):
        data = self.fs.read(self.journal_path)
        if data is None:
            return None
        value = decode_json(data)
        require(isinstance(value, dict) and set(value) == {"schemaVersion", "target", "operation", "phase", "transaction", "before", "after", "changes"}
                and value["schemaVersion"] == 1 and value["target"] == self.target
                and value["operation"] in ("install", "update", "uninstall")
                and value["phase"] in ("preparing", "prepared", "stopped", "committed", "ready", "repair-required"), "invalid maintenance journal")
        self.validate_registry(value["before"])
        self.validate_registry(value["after"])
        transaction = value["transaction"]
        require(isinstance(transaction, str) and re.fullmatch(r"txn-[a-f0-9]{32}", transaction), "invalid journal transaction")
        expected = set(value["before"]["files"]) | set(value["after"]["files"])
        require(isinstance(value["changes"], dict) and set(value["changes"]) == expected, "journal changed the fixed inventory")
        for index, path in enumerate(sorted(expected)):
            record = value["changes"][path]
            require(isinstance(record, dict) and set(record) == {"before", "after"}, "invalid journal file entry")
            for phase, prefix, registry in (("before", "old", value["before"]), ("after", "new", value["after"])):
                original = registry["files"].get(path)
                expected_record = None if original is None else {**original, "staged": self.paths["state"] + f"/{transaction}/{prefix}-{index}"}
                require(record[phase] == expected_record, "journal contains an unexpected path or hash")
        return value

    def _blocked(self):
        self.fs.write(self.paths["marker"], b"ROKBattlesCaptureMaintenance:1\n", 0o644)

    def _quiesce(self, users):
        self._blocked()
        identities = self.images()
        agent = {path: identity for path, identity in identities.items() if path == self.paths["agent"]}
        self.services.wait_images_absent(agent)
        for uid in sorted(users):
            self.services.stop(uid)
        self.services.wait_images_absent(identities)

    def _staged_bytes(self, record):
        data = self.fs.read(record["staged"], MAX_BINARY)
        require(data is not None and sha(data) == record["sha256"], "staged recovery image changed")
        return data

    def _cleanup(self, journal):
        # Only exact validated file names. Do not recursively remove an arbitrary
        # root-owned directory or anything named by an unchecked JSON value.
        for record in journal["changes"].values():
            for phase in ("before", "after"):
                if record[phase] is not None:
                    self.fs.delete(record[phase]["staged"])
        directory = self.paths["state"] + "/" + journal["transaction"]
        with self.fs.parent(directory) as (parent, name):
            try:
                descriptor = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=parent)
                self.fs.check(descriptor, True)
                os.close(descriptor)
                os.rmdir(name, dir_fd=parent)
                os.fsync(parent)
            except FileNotFoundError:
                pass
        self.fs.delete(self.journal_path)

    def _restart(self, registry):
        self.validate_registry(registry, True)
        self.services.reload()
        # Unix helpers reject even startup while the block is present. Clear only
        # after the complete verified binary/registry commit, before explicit starts.
        self.fs.delete(self.paths["marker"])
        try:
            for uid in registry["uids"]:
                self.services.start(uid)
        except Exception:
            self._blocked()
            raise

    def run(self, operation, package=None, uid=None, all_registered=False):
        require(operation in ("install", "update", "uninstall", "repair"), "unknown maintenance operation")
        if uid is not None:
            uid = uid_value(uid)
        with self.fs.lock(self.paths["state"] + "/lock"):
            pending = self._journal()
            if operation == "repair":
                require(pending is not None, "no interrupted maintenance transaction")
                return self.repair(pending, all_registered)
            require(pending is None, "interrupted maintenance requires explicit repair")
            require(self.fs.read(self.paths["marker"]) is None, "unexplained maintenance barrier requires administrator repair")
            before = self.registry()
            self.services.assert_inventory(before["uids"])
            users = set(before["uids"])
            if operation == "install":
                require(package is not None and package.target == self.target and uid is not None, "install requires a release package and selected UID")
                users.add(uid)
            elif operation == "update":
                require(package is not None and package.target == self.target and users and uid is None, "update requires an existing installation")
            else:
                require(uid in users and package is None, "uninstall requires one registered UID")
                users.remove(uid)
            require(len(users) <= MAX_USERS, "too many registered users")
            affected = set(before["uids"]) | users
            require(len(affected) <= 1 or all_registered, "shared images affect other registered users; explicit all-user confirmation required")
            after = {"schemaVersion": 1, "target": self.target, "uids": sorted(users), "files": {},
                     "release": dict(self.pins) if package is not None else before["release"]}
            payload = {}
            if users:
                if package is not None:
                    for role in ("helper", "agent"):
                        payload[self.paths[role]] = (package.files[self.paths[role]], 0o755)
                    for selected in sorted(users):
                        payload[service_path(self.target, selected)] = (definition(package, selected), 0o644)
                    if operation == "install" and before["uids"]:
                        for path, record in before["files"].items():
                            if path in payload:
                                require(sha(payload[path][0]) == record["sha256"], "adding a user cannot silently update shared images")
                else:
                    wanted = {self.paths["helper"], self.paths["agent"]} | {service_path(self.target, selected) for selected in users}
                    payload = {path: (self.fs.read(path, MAX_BINARY), before["files"][path]["mode"]) for path in wanted}
                after["files"] = {path: {"sha256": sha(data), "mode": mode} for path, (data, mode) in payload.items()}
            self.validate_registry(after)
            transaction = "txn-" + uuid.uuid4().hex
            changes = {}
            for index, path in enumerate(sorted(set(before["files"]) | set(after["files"]))):
                if path not in before["files"]:
                    require(self.fs.read(path, MAX_BINARY) is None, "unmanaged file conflicts with installation")
                changes[path] = {}
                for phase, prefix, registry in (("before", "old", before), ("after", "new", after)):
                    record = registry["files"].get(path)
                    changes[path][phase] = None if record is None else {**record, "staged": self.paths["state"] + f"/{transaction}/{prefix}-{index}"}
            journal = {"schemaVersion": 1, "target": self.target, "operation": operation, "phase": "preparing",
                       "transaction": transaction, "before": before, "after": after, "changes": changes}
            self.persist(self.journal_path, journal)
            try:
                for path, record in changes.items():
                    if record["before"] is not None:
                        data = self.fs.read(path, MAX_BINARY)
                        require(data is not None and sha(data) == record["before"]["sha256"], "original image changed before backup")
                        self.fs.write(record["before"]["staged"], data, record["before"]["mode"])
                    if record["after"] is not None:
                        self.fs.write(record["after"]["staged"], payload[path][0], record["after"]["mode"])
                if package is not None:
                    self.services.validate_signatures({role: self.fs.absolute(changes[self.paths[role]]["after"]["staged"]) for role in ("helper", "agent")}, package.team)
                journal["phase"] = "prepared"
                self.persist(self.journal_path, journal)
                self._quiesce(affected)
                journal["phase"] = "stopped"
                self.persist(self.journal_path, journal)
                for path, record in changes.items():
                    if record["after"] is None:
                        self.fs.delete(path)
                    else:
                        self.fs.write(path, self._staged_bytes(record["after"]), record["after"]["mode"])
                self.validate_registry(after, True)
                self.persist(self.registry_path, after)
                journal["phase"] = "committed"
                self.persist(self.journal_path, journal)
                self._restart(after)
                journal["phase"] = "ready"
                self.persist(self.journal_path, journal)
                self._cleanup(journal)
            except Exception:
                self._blocked()
                # Preserve the actual phase: repair must know whether any live
                # payload may have changed. Never claim a partial backup complete.
                self.persist(self.journal_path, journal)
                raise
            return after

    def repair(self, journal, all_registered):
        before, after = journal["before"], journal["after"]
        users = set(before["uids"]) | set(after["uids"])
        require(len(users) <= 1 or all_registered, "repair affects all registered users")
        self._quiesce(users)
        if journal["phase"] == "preparing":
            # No installed byte changed before the prepared marker was committed.
            self.validate_registry(before, True)
        elif journal["phase"] == "ready":
            # Cleanup may have been interrupted after successful activation. Keep
            # the completely verified new generation instead of missing old backups.
            self.validate_registry(after, True)
            self.persist(self.registry_path, after)
            self._restart(after)
            self._cleanup(journal)
            return after
        else:
            originals = {path: self._staged_bytes(record["before"]) for path, record in journal["changes"].items() if record["before"] is not None}
            for path, record in journal["changes"].items():
                if record["before"] is None:
                    self.fs.delete(path)
                else:
                    self.fs.write(path, originals[path], record["before"]["mode"])
            self.validate_registry(before, True)
        if before["uids"] and self.target.endswith("apple-darwin"):
            self.services.validate_signatures({role: self.fs.absolute(self.paths[role]) for role in ("helper", "agent")}, before["release"]["appleTeamId"])
        self.persist(self.registry_path, before)
        self._restart(before)
        self._cleanup(journal)
        return before


def running_target():
    family = {"Linux": "unknown-linux-gnu", "Darwin": "apple-darwin"}.get(platform.system())
    machine = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "aarch64", "arm64": "aarch64"}.get(platform.machine().lower())
    require(family is not None and machine is not None, "unsupported native maintenance host")
    return machine + "-" + family


def uid_argument(value):
    try:
        return uid_value(value)
    except MaintenanceError as error:
        raise argparse.ArgumentTypeError("invalid local UID") from error


def parser():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("operation", choices=("install", "update", "uninstall", "repair"))
    arguments.add_argument("--package", type=Path)
    arguments.add_argument("--uid", type=uid_argument)
    arguments.add_argument("--confirm-system-changes", action="store_true", help="explicit administrator consent for the selected fixed installation")
    arguments.add_argument("--all-registered-users", action="store_true", help="confirm interruption of every registered capture user while shared images change")
    return arguments


def main():
    args = parser().parse_args()
    try:
        require(os.geteuid() == 0 and sys.flags.isolated, "run the trusted release installer explicitly as administrator with Python -I; automatic elevation is unsupported")
        require(args.confirm_system_changes, "explicit administrator confirmation is required")
        # The release code is itself an authority. Refuse a still-user-writable
        # invocation even when its payload hashes are embedded. A trusted outer
        # package/admin review must provision this script before root invocation.
        script = Path(__file__).absolute()
        require(script.is_absolute(), "installer path unavailable")
        FileSystem().read(script.relative_to(Path("/")).as_posix(), MAX_METADATA)
        target = running_target()
        pins = EMBEDDED_RELEASE
        require(isinstance(pins, dict) and pins.get("target") == target, "installer is not bound to a trusted release for this host")
        if args.uid is not None:
            try:
                pwd.getpwuid(args.uid)
            except KeyError as error:
                raise MaintenanceError("selected local user does not exist") from error
        require((args.operation in ("install", "uninstall")) == (args.uid is not None), "choose a user only for install or uninstall")
        require((args.operation in ("install", "update")) == (args.package is not None), "package is required only for install or update")
        package = Package.read(args.package, pins) if args.package is not None else None
        fs = FileSystem()
        installer = Installer(fs, NativeServices(fs, target), target, pins)
        installer.run(args.operation, package, args.uid, args.all_registered_users)
        print("ROK Battles maintenance completed. Restart the ordinary app to resume its desired worker setting; capture still requires user opt-in.")
    except (MaintenanceError, OSError, subprocess.SubprocessError, UnicodeError, ValueError) as error:
        message = str(error) if isinstance(error, MaintenanceError) else "native operation failed; any existing maintenance block was retained"
        parser().exit(1, "Maintenance stopped: " + message + "\n")


if __name__ == "__main__":
    main()
