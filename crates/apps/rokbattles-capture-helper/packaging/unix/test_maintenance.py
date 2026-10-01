"""Filesystem/lifecycle fixtures only. Never touch a system service or native capture."""
import contextlib
import io
import json
import os
from pathlib import Path
import stat
import struct
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

if os.name == "nt":
    raise unittest.SkipTest("Unix filesystem fixture suite")

import maintain
import stage_package

LINUX = "x86_64-unknown-linux-gnu"
MAC = "aarch64-apple-darwin"


def binary(target=LINUX, tag=0):
    data = bytearray(128)
    if target.endswith("linux-gnu"):
        data[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<HH", data, 16, 3, stage_package.TARGETS[target][1])
    else:
        data[:4] = b"\xcf\xfa\xed\xfe"
        struct.pack_into("<I", data, 4, stage_package.TARGETS[target][1])
        struct.pack_into("<I", data, 12, 2)
    data[-1] = tag
    return bytes(data)


class FakeServices:
    def __init__(self, fs, target=LINUX):
        self.fs, self.paths = fs, maintain.layout(target)
        self.events, self.unknown = [], set()
        self.fail_start = False
        self.busy = False
        self.crash = False

    def assert_inventory(self, users):
        self.events.append(("inventory", tuple(users)))
        maintain.require(self.unknown <= set(users), "unregistered capture service conflicts with maintenance")

    def wait_images_absent(self, identities):
        self.events.append(("wait", tuple(sorted(identities))))
        assert self.fs.read(self.paths["marker"]) is not None
        if self.crash:
            self.crash = False
            raise KeyboardInterrupt("synthetic installer death")
        if self.busy:
            raise maintain.MaintenanceError("synthetic busy process")

    def stop(self, uid):
        assert self.fs.read(self.paths["marker"]) is not None
        self.events.append(("stop", uid))

    def reload(self):
        self.events.append(("reload",))

    def start(self, uid):
        assert self.fs.read(self.paths["marker"]) is None
        self.events.append(("start", uid))
        if self.fail_start:
            self.fail_start = False
            raise maintain.MaintenanceError("synthetic failed readiness")

    def validate_signatures(self, staged, team):
        self.events.append(("signatures", team))
        assert set(staged) == {"helper", "agent"}
        assert all(path.is_file() for path in staged.values())


class FailingFileSystem(maintain.FileSystem):
    fail_path = None

    def write(self, relative, data, mode=0o644):
        if relative == self.fail_path:
            self.fail_path = None
            raise OSError("synthetic interrupted replace")
        return super().write(relative, data, mode)


class MaintenanceFixtures(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / "root"
        self.root.mkdir(mode=0o700)
        self.fs = FailingFileSystem(self.root, owner=os.geteuid())
        self.services = FakeServices(self.fs)
        self.sequence = 0
        self.package, self.pins, self.archive = self.release()
        self.install = self.installer()

    def release(self, target=LINUX, tag=0):
        self.sequence += 1
        directory = self.base / f"release-{self.sequence}"
        directory.mkdir()
        helper, agent = directory / "helper", directory / "agent"
        helper.write_bytes(binary(target, tag))
        agent.write_bytes(binary(target, tag + 1))
        archive = directory / "capture.tar.gz"
        stage_package.stage(helper, agent, target, archive)
        pins = {"schemaVersion": 1, "target": target, "archiveSha256": maintain.sha(archive.read_bytes()),
                "archiveName": archive.name, "appleTeamId": "ABCDE12345" if target.endswith("darwin") else None}
        return maintain.Package.read(archive, pins), pins, archive

    def installer(self, package=None, pins=None):
        return maintain.Installer(self.fs, self.services, LINUX, pins or self.pins)

    def test_first_install_is_ordered_and_leaves_only_verified_ready_state(self):
        after = self.install.run("install", self.package, 1001)
        self.assertEqual(after["uids"], [1001])
        self.assertIsNone(self.fs.read(self.install.journal_path))
        self.assertIsNone(self.fs.read(self.install.paths["marker"]))
        for path, record in after["files"].items():
            self.assertEqual(maintain.sha(self.fs.read(path, maintain.MAX_BINARY)), record["sha256"])
            self.assertEqual(stat.S_IMODE(self.fs.metadata(path).st_mode), record["mode"])
        events = [event[0] for event in self.services.events]
        self.assertLess(events.index("wait"), events.index("stop"))
        self.assertLess(events.index("stop"), events.index("start"))

    def test_shared_update_requires_all_user_authorization_and_drains_every_helper(self):
        self.install.run("install", self.package, 1001)
        with self.assertRaises(maintain.MaintenanceError):
            self.install.run("install", self.package, 1002)
        self.install.run("install", self.package, 1002, all_registered=True)
        newer, pins, _ = self.release(tag=9)
        self.services.events.clear()
        updated = self.installer(pins=pins).run("update", newer, all_registered=True)
        self.assertEqual(updated["uids"], [1001, 1002])
        self.assertEqual([event[1] for event in self.services.events if event[0] == "stop"], [1001, 1002])
        self.assertEqual([event[1] for event in self.services.events if event[0] == "start"], [1001, 1002])

    def test_add_user_cannot_hide_a_shared_binary_upgrade(self):
        self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        with self.assertRaisesRegex(maintain.MaintenanceError, "silently update"):
            self.installer(pins=pins).run("install", newer, 1002, all_registered=True)
        self.assertEqual(self.install.registry()["uids"], [1001])

    def test_remove_one_user_preserves_shared_images_until_last_uninstall(self):
        self.install.run("install", self.package, 1001)
        self.install.run("install", self.package, 1002, all_registered=True)
        self.install.run("uninstall", uid=1001, all_registered=True)
        self.assertIsNotNone(self.fs.read(self.install.paths["helper"], maintain.MAX_BINARY))
        self.assertEqual(self.install.registry()["uids"], [1002])
        self.install.run("uninstall", uid=1002)
        self.assertIsNone(self.fs.read(self.install.paths["helper"], maintain.MAX_BINARY))
        self.assertEqual(self.install.registry()["uids"], [])

    def test_partial_update_retains_barrier_and_restores_all_old_images(self):
        original = self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        self.fs.fail_path = self.install.paths["agent"]
        with self.assertRaises(OSError):
            self.installer(pins=pins).run("update", newer)
        self.assertIsNotNone(self.fs.read(self.install.paths["marker"]))
        self.assertEqual(self.install._journal()["phase"], "stopped")
        repaired = self.install.run("repair")
        self.assertEqual(repaired, original)
        self.assertIsNone(self.fs.read(self.install.paths["marker"]))
        self.assertIsNone(self.install._journal())

    def test_failed_readiness_blocks_and_repair_rolls_back(self):
        original = self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        self.services.fail_start = True
        with self.assertRaises(maintain.MaintenanceError):
            self.installer(pins=pins).run("update", newer)
        self.assertIsNotNone(self.fs.read(self.install.paths["marker"]))
        self.assertEqual(self.install.run("repair"), original)

    def test_first_install_rollback_removes_files_originally_absent(self):
        self.fs.fail_path = self.install.paths["agent"]
        with self.assertRaises(OSError):
            self.install.run("install", self.package, 1001)
        result = self.install.run("repair")
        self.assertEqual(result["uids"], [])
        self.assertIsNone(self.fs.read(self.install.paths["helper"], maintain.MAX_BINARY))
        self.assertIsNone(self.fs.read(maintain.service_path(LINUX, 1001)))

    def test_installer_death_leaves_verified_journal_for_explicit_repair(self):
        self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        self.services.crash = True
        with self.assertRaises(KeyboardInterrupt):
            self.installer(pins=pins).run("update", newer)
        self.assertIsNotNone(self.fs.read(self.install.paths["marker"]))
        with self.assertRaisesRegex(maintain.MaintenanceError, "explicit repair"):
            self.install.run("update", self.package)
        self.install.run("repair")
        self.assertIsNone(self.install._journal())

    def test_busy_image_never_changes_installed_payload(self):
        original = self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        self.services.busy = True
        with self.assertRaises(maintain.MaintenanceError):
            self.installer(pins=pins).run("update", newer)
        self.assertEqual(self.install.registry(), original)
        self.assertIsNotNone(self.fs.read(self.install.paths["marker"]))

    def test_tampered_backup_cannot_be_restored_or_unblock_capture(self):
        self.install.run("install", self.package, 1001)
        newer, pins, _ = self.release(tag=9)
        self.fs.fail_path = self.install.paths["agent"]
        with self.assertRaises(OSError):
            self.installer(pins=pins).run("update", newer)
        journal = self.install._journal()
        record = journal["changes"][self.install.paths["helper"]]["before"]
        self.fs.write(record["staged"], b"changed")
        with self.assertRaisesRegex(maintain.MaintenanceError, "changed"):
            self.install.run("repair")
        self.assertIsNotNone(self.fs.read(self.install.paths["marker"]))

    def test_unknown_service_conflict_has_no_payload_side_effects(self):
        self.services.unknown.add(1009)
        with self.assertRaisesRegex(maintain.MaintenanceError, "unregistered"):
            self.install.run("install", self.package, 1001)
        self.assertIsNone(self.fs.read(self.install.paths["helper"], maintain.MAX_BINARY))
        self.assertIsNone(self.install._journal())

    def test_second_maintenance_process_cannot_enter_the_transaction(self):
        with self.fs.lock(self.install.paths["state"] + "/lock"):
            with self.assertRaisesRegex(maintain.MaintenanceError, "another maintenance"):
                self.install.run("install", self.package, 1001)

    def test_symlinks_writable_files_setuid_and_fifo_fail_closed(self):
        outside = self.base / "outside"
        outside.write_bytes(b"preserve")
        location = self.install.paths["marker"]
        self.fs.write(location, b"fixture")
        path = self.fs.absolute(location)
        path.unlink()
        path.symlink_to(outside)
        with self.assertRaises(OSError):
            self.fs.write(location, b"bad")
        self.assertEqual(outside.read_bytes(), b"preserve")
        path.unlink()
        self.fs.write(location, b"fixture")
        for mode in (0o666, 0o4644):
            path.chmod(mode)
            with self.assertRaises(maintain.MaintenanceError):
                self.fs.read(location)
        path.unlink()
        os.mkfifo(path, 0o600)
        with self.assertRaises(maintain.MaintenanceError):
            self.fs.read(location)

    def test_embedded_pin_cannot_be_overridden_by_environment_or_cli(self):
        with patch.dict(os.environ, {"ROKBATTLES_CAPTURE_SHA256": self.pins["archiveSha256"], "ROKBATTLES_CAPTURE_TARGET": LINUX}):
            with self.assertRaises(maintain.MaintenanceError):
                maintain.Package.read(self.archive, maintain.EMBEDDED_RELEASE)
        for extra in ("--sha256", "--team-id", "--target", "--root"):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                maintain.parser().parse_args(["install", "--uid", "1001", extra, "untrusted"])

    def test_wrong_digest_name_target_and_linked_archive_are_rejected(self):
        wrong = {**self.pins, "archiveSha256": "0" * 64}
        with self.assertRaises(maintain.MaintenanceError):
            maintain.Package.read(self.archive, wrong)
        with self.assertRaises(maintain.MaintenanceError):
            maintain.Package.read(self.archive, {**self.pins, "archiveName": "different.tar.gz"})
        link = self.archive.parent / "linked.tar.gz"
        link.symlink_to(self.archive)
        with self.assertRaises(OSError):
            maintain.Package.read(link, {**self.pins, "archiveName": link.name})
        with self.assertRaises(maintain.MaintenanceError):
            maintain.Package.read(self.archive, {**self.pins, "target": "aarch64-unknown-linux-gnu"})

    def test_macos_template_is_uid_neutral_until_explicit_install_and_strict(self):
        package, _, _ = self.release(MAC)
        import plistlib
        value = plistlib.loads(maintain.definition(package, 501))
        self.assertEqual(value["Label"], "com.rokbattles.capture-helper.501")
        self.assertEqual(value["ProgramArguments"][-1], "501")
        template = plistlib.loads(package.files[package.paths["template"]])
        template["Umask"] = 0
        package.files[package.paths["template"]] = plistlib.dumps(template)
        with self.assertRaisesRegex(maintain.MaintenanceError, "unsafe launchd"):
            maintain.definition(package, 501)


if __name__ == "__main__":
    unittest.main()
