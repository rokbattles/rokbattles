"""Synthetic package tests. No root, capture, binary execution or service actions."""
import json
from pathlib import Path
import plistlib
import struct
import tarfile
import tempfile
import unittest

import stage_package as package


def binary(target):
    platform, machine = package.TARGETS[target]
    data = bytearray(64)
    if platform == "linux":
        data[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<HH", data, 16, 3, machine)
    else:
        data[:4] = b"\xcf\xfa\xed\xfe"
        struct.pack_into("<I", data, 4, machine)
        struct.pack_into("<I", data, 12, 2)
    return bytes(data)


class PackageTests(unittest.TestCase):
    def test_macos_template_and_manifest_require_installer_user_selection(self):
        target = "aarch64-apple-darwin"
        files = package.payloads(binary(target), binary(target), target)
        manifest = json.loads(files["manifest.json"][0])
        self.assertNotIn("uid", manifest)
        self.assertTrue(manifest["requiresInstallerUserSelection"])
        plist = plistlib.loads(files["share/rokbattles-capture/com.rokbattles.capture-helper.plist.in"][0])
        self.assertEqual(plist["Label"], "com.rokbattles.capture-helper.@UID@")
        self.assertEqual(plist["ProgramArguments"][-1], "@UID@")

    def test_all_four_targets_have_fixed_executable_and_scoped_service(self):
        for target in package.TARGETS:
            with self.subTest(target=target):
                files = package.payloads(binary(target), binary(target), target)
                manifest = json.loads(files["manifest.json"][0])
                self.assertFalse(manifest["installPerformed"])
                self.assertTrue(manifest["administratorConsentRequired"])
                self.assertTrue(manifest["signingAndNativeValidationRequired"])
                executable = [path for path, (_, mode) in files.items() if mode == 0o755]
                self.assertEqual(len(executable), 2)
                self.assertTrue(all(".." not in Path(path).parts and not path.startswith("/") for path in files))
                if "apple" in target:
                    plist = plistlib.loads(files["share/rokbattles-capture/com.rokbattles.capture-helper.plist.in"][0])
                    self.assertEqual(plist["ProgramArguments"], ["/Library/PrivilegedHelperTools/com.rokbattles.capture-helper", "--service", "--uid", "@UID@"])
                    self.assertEqual(plist["Umask"], 0o077)
                    self.assertEqual(plist["UserName"], "root")
                else:
                    service = files["usr/lib/systemd/system/rokbattles-capture@.service"][0].decode()
                    self.assertIn("--service --uid %i", service)
                    self.assertIn("NoNewPrivileges=yes", service)
                    self.assertNotIn("AmbientCapabilities=", service)

    def test_wrong_binary_architecture_or_format_is_rejected(self):
        for target in package.TARGETS:
            for other in package.TARGETS:
                if target != other:
                    with self.assertRaises(ValueError):
                        package.check_binary(binary(other), target)
            with self.assertRaises(ValueError):
                package.check_binary(b"not an executable", target)

    def test_tar_is_only_regular_root_owned_files_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "synthetic-helper"
            source.write_bytes(binary("x86_64-unknown-linux-gnu"))
            output = root / "package.tar.gz"
            package.stage(source, source, "x86_64-unknown-linux-gnu", output)
            with tarfile.open(output) as archive:
                self.assertTrue(all(member.isfile() and member.uid == 0 and member.gid == 0 for member in archive.getmembers()))
            with self.assertRaises(FileExistsError):
                package.stage(source, source, "x86_64-unknown-linux-gnu", output)


if __name__ == "__main__":
    unittest.main()
