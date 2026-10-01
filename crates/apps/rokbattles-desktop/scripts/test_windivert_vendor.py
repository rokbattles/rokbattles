"""Synthetic tests only: no vendor code is loaded, installed or executed."""

import copy
import io
import json
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import warnings
import zipfile

import windivert_vendor as vendor


class VendorTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "resources"
        self.payload = {
            "WinDivert.dll": b"synthetic unsigned DLL, not executable",
            "WinDivert64.sys": b"synthetic driver, not executable",
            "LICENSE": b"synthetic license",
            "README": b"synthetic readme",
            "VERSION": b"2.2.2\n",
        }
        self.lock = copy.deepcopy(vendor.load_lock())
        self.lock["members"] = {
            name: {"path": "upstream/" + name, "size": len(data), "sha256": vendor.digest(data)}
            for name, data in self.payload.items()
        }

    def archive(self, extras=(), replace=None):
        stream = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(stream, "w", zipfile.ZIP_DEFLATED) as archive:
                for name, data in self.payload.items():
                    archive.writestr("upstream/" + name, (replace or {}).get(name, data))
                for name, data in extras:
                    archive.writestr(name, data)
        data = stream.getvalue()
        self.lock["archive"]["sha256"] = vendor.digest(data)
        self.lock["archive"]["size"] = len(data)
        return data

    def test_stages_exact_allowlist_and_manifest(self):
        data = self.archive(extras=[("upstream/x86/unused.exe", b"must not be written")])
        vendor.stage(data, self.lock, self.root)
        self.assertEqual(
            {path.name for path in self.root.iterdir()},
            set(self.payload) | {"NOTICE.txt", "SOURCE.txt", "manifest.json"},
        )
        for name, content in self.payload.items():
            self.assertEqual((self.root / name).read_bytes(), content)
        vendor.verify(self.lock, self.root)

    def test_whole_archive_hash_is_required_before_staging(self):
        data = self.archive()
        with self.assertRaisesRegex(vendor.VerificationError, "Archive size or SHA"):
            vendor.stage(data[:-1] + bytes([data[-1] ^ 1]), self.lock, self.root)
        self.assertFalse(self.root.exists())

    def test_whole_archive_size_is_pinned(self):
        data = self.archive()
        self.lock["archive"]["size"] += 1
        with self.assertRaises(vendor.VerificationError):
            vendor.unpack_verified(data, self.lock)

    def test_independent_member_hash_is_required(self):
        data = self.archive(replace={"LICENSE": b"x" * len(self.payload["LICENSE"])})
        with self.assertRaisesRegex(vendor.VerificationError, "Incorrect member SHA"):
            vendor.unpack_verified(data, self.lock)

    def test_member_size_is_pinned(self):
        data = self.archive(replace={"LICENSE": b"too long" * 10})
        with self.assertRaisesRegex(vendor.VerificationError, "Incorrect member size"):
            vendor.unpack_verified(data, self.lock)

    def test_missing_member_is_rejected(self):
        data = self.archive()
        self.lock["members"]["LICENSE"]["path"] = "upstream/absent"
        with self.assertRaisesRegex(vendor.VerificationError, "Missing pinned member"):
            vendor.unpack_verified(data, self.lock)

    def test_duplicate_and_case_colliding_members_are_rejected(self):
        for name in ["upstream/LICENSE", "upstream/license", "upstream/LICENSE/"]:
            with self.subTest(name=name):
                data = self.archive(extras=[(name, b"")])
                with self.assertRaises(vendor.VerificationError):
                    vendor.unpack_verified(data, self.lock)

    def test_unsafe_unselected_paths_are_rejected(self):
        for name in ["../escape", "/absolute", "C:/drive", "upstream/../escape", "a:b", "a//b", "a/./b", "a/b.", "a/b ", "a//", "a/CON", "a/nul.txt", "a/LPT1"]:
            with self.subTest(name=name):
                data = self.archive(extras=[(name, b"unused")])
                with self.assertRaisesRegex(vendor.VerificationError, "Unsafe archive member"):
                    vendor.unpack_verified(data, self.lock)

    def test_raw_backslash_member_is_rejected_on_every_host(self):
        # ZipInfo normalizes backslashes on Windows while writing, so mutate
        # the two on-disk filename fields to preserve the malicious raw name.
        data = self.archive(extras=[("unsafe/entry", b"unused")])
        data = data.replace(b"unsafe/entry", b"unsafe\\entry")
        self.lock["archive"]["sha256"] = vendor.digest(data)
        with self.assertRaisesRegex(vendor.VerificationError, "Unsafe archive member"):
            vendor.unpack_verified(data, self.lock)

    def test_safe_directory_returns_canonical_identity(self):
        self.root.mkdir()
        self.assertEqual(vendor.safe_directory(self.root), self.root.resolve())

    def test_unsafe_output_names_are_rejected(self):
        data = self.archive()
        for name in ["../escape", "CON", "NUL.txt", "COM1", "LPT9", "file."]:
            with self.subTest(name=name):
                lock = copy.deepcopy(self.lock)
                lock["members"][name] = lock["members"].pop("LICENSE")
                with self.assertRaisesRegex(vendor.VerificationError, "Unsafe output name"):
                    vendor.unpack_verified(data, lock)

    def test_symlinks_and_special_zip_entries_are_rejected(self):
        for mode in [stat.S_IFLNK, stat.S_IFIFO, stat.S_IFCHR, stat.S_IFSOCK]:
            with self.subTest(mode=mode):
                info = zipfile.ZipInfo("upstream/other")
                info.external_attr = (mode | 0o777) << 16
                data = self.archive(extras=[(info, b"target")])
                with self.assertRaisesRegex(vendor.VerificationError, "Non-regular archive"):
                    vendor.unpack_verified(data, self.lock)

    def test_encrypted_and_nul_entries_are_rejected(self):
        info = zipfile.ZipInfo("upstream/other")
        info.flag_bits = 1
        with self.assertRaisesRegex(vendor.VerificationError, "Encrypted"):
            vendor.validate_member(info)
        info = zipfile.ZipInfo("upstream/other\0hidden")
        with self.assertRaisesRegex(vendor.VerificationError, "Unsafe archive"):
            vendor.validate_member(info)

    def test_expansion_and_entry_limits(self):
        data = self.archive()
        with mock.patch.object(vendor, "MAX_EXPANDED_BYTES", 1):
            with self.assertRaisesRegex(vendor.VerificationError, "expands beyond"):
                vendor.unpack_verified(data, self.lock)
        with mock.patch.object(vendor, "MAX_ENTRIES", 1):
            with self.assertRaisesRegex(vendor.VerificationError, "expands beyond"):
                vendor.unpack_verified(data, self.lock)

    def test_bounded_reads_stop_at_limit(self):
        with self.assertRaises(vendor.VerificationError):
            vendor.read_bounded(io.BytesIO(b"12345"), 4)
        self.assertEqual(vendor.read_bounded(io.BytesIO(b"1234"), 4), b"1234")

    def test_version_is_checked_even_with_member_hash(self):
        self.payload["VERSION"] = b"9.9.9\n"
        self.lock["members"]["VERSION"]["sha256"] = vendor.digest(self.payload["VERSION"])
        with self.assertRaisesRegex(vendor.VerificationError, "Unexpected upstream version"):
            vendor.unpack_verified(self.archive(), self.lock)

    def test_restage_replaces_old_files_and_removes_only_named_stale_files(self):
        data = self.archive()
        vendor.stage(data, self.lock, self.root)
        (self.root / "WinDivert32.sys").write_bytes(b"old driver")
        (self.root / "WinDivert.lib").write_bytes(b"unused old import lib")
        (self.root / "WinDivert.dll").write_bytes(b"stale")
        # A previous manifest cannot authorize removal outside the allowlist.
        outside = self.root.parent / "keep.txt"
        outside.write_bytes(b"keep")
        (self.root / "manifest.json").write_text('{"files":{"../keep.txt":{}}}')
        vendor.stage(data, self.lock, self.root)
        self.assertFalse((self.root / "WinDivert32.sys").exists())
        self.assertFalse((self.root / "WinDivert.lib").exists())
        self.assertEqual(outside.read_bytes(), b"keep")
        vendor.verify(self.lock, self.root)

    def test_unknown_stale_files_abort_without_cleanup(self):
        data = self.archive()
        vendor.stage(data, self.lock, self.root)
        (self.root / "unrelated.txt").write_bytes(b"keep")
        with self.assertRaisesRegex(vendor.VerificationError, "Unexpected file"):
            vendor.stage(data, self.lock, self.root)
        self.assertEqual((self.root / "unrelated.txt").read_bytes(), b"keep")
        with self.assertRaises(vendor.VerificationError):
            vendor.verify(self.lock, self.root)

    def test_partial_stage_has_no_valid_manifest(self):
        data = self.archive()
        vendor.stage(data, self.lock, self.root)
        with mock.patch.object(vendor, "atomic_write", side_effect=OSError("interrupted")):
            with self.assertRaises(OSError):
                vendor.stage(data, self.lock, self.root)
        self.assertFalse((self.root / "manifest.json").exists())
        with self.assertRaises(OSError):
            vendor.verify(self.lock, self.root)

    def test_tampered_manifest_and_payload_fail(self):
        data = self.archive()
        vendor.stage(data, self.lock, self.root)
        (self.root / "manifest.json").write_text("{}")
        with self.assertRaises(vendor.VerificationError):
            vendor.verify(self.lock, self.root)
        vendor.stage(data, self.lock, self.root)
        (self.root / "WinDivert64.sys").write_bytes(b"wrong driver")
        with self.assertRaises(vendor.VerificationError):
            vendor.verify(self.lock, self.root)

    def test_missing_file_fails(self):
        vendor.stage(self.archive(), self.lock, self.root)
        (self.root / "LICENSE").unlink()
        with self.assertRaises(OSError):
            vendor.verify(self.lock, self.root)

    def make_symlink(self, destination, target, directory=False):
        try:
            destination.symlink_to(target, target_is_directory=directory)
        except OSError as error:
            self.skipTest(f"Symlinks unavailable on this runner: {error}")

    def test_symlink_ancestor_is_rejected(self):
        target = self.root.parent / "target"
        target.mkdir()
        self.make_symlink(self.root, target, directory=True)
        with self.assertRaisesRegex(vendor.VerificationError, "Symlink or junction"):
            vendor.stage(self.archive(), self.lock, self.root / "nested")
        self.assertEqual(list(target.iterdir()), [])

    def test_symlink_output_is_rejected_without_touching_target(self):
        self.root.mkdir()
        target = self.root.parent / "external"
        target.write_bytes(b"keep")
        self.make_symlink(self.root / "WinDivert.dll", target)
        with self.assertRaisesRegex(vendor.VerificationError, "Unexpected file or link"):
            vendor.stage(self.archive(), self.lock, self.root)
        self.assertEqual(target.read_bytes(), b"keep")

    def test_arm64_rejected_before_download(self):
        result = subprocess.run(
            [sys.executable, str(Path(vendor.__file__)), "stage", "--target", "aarch64-pc-windows-msvc"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("never bundle on ARM64", result.stderr)

    def test_exact_x64_bundle_map_and_per_machine_installer(self):
        overlay = json.loads((vendor.DESKTOP / "tauri.windows-x64.conf.json").read_text())
        bundle = overlay["bundle"]
        names = set(vendor.load_lock()["members"]) | set(vendor.NOTICES) | {"manifest.json"}
        self.assertEqual(bundle["resources"], {
            f"vendor/windivert/{vendor.TARGET}/{name}": f"capture/windivert/{vendor.TARGET}/{name}"
            for name in names
        })
        self.assertEqual(bundle["windows"]["nsis"]["installMode"], "perMachine")
        default = json.loads((vendor.DESKTOP / "tauri.conf.json").read_text())
        self.assertNotIn("resources", default["bundle"])
        self.assertFalse((vendor.DESKTOP / "tauri.windows.conf.json").exists())

    def test_only_x64_release_matrix_uses_vendor_overlay(self):
        release = (vendor.DESKTOP.parents[2] / ".github/workflows/release.yml").read_text()
        x64 = next(line for line in release.splitlines() if 'args: "--target x86_64-pc-windows-msvc' in line)
        arm64 = next(line for line in release.splitlines() if 'args: "--target aarch64-pc-windows-msvc' in line)
        self.assertIn("--config tauri.windows-x64.conf.json", x64)
        self.assertNotIn("--config", arm64)
        self.assertEqual(release.count("--config tauri.windows-x64.conf.json"), 1)

    def test_nested_kernel_signature_index_is_pinned_and_manifested(self):
        lock = vendor.load_lock()
        self.assertEqual(lock["kernelSignatureIndex"], 1)
        manifest = json.loads(vendor.expected_manifest(lock, vendor.notices()))
        self.assertEqual(manifest["kernelSignatureIndex"], 1)
        script = (vendor.DESKTOP / "scripts/verify_windivert.ps1").read_text()
        self.assertIn("verify /pa /ds $KernelSignatureIndex /tw /v $Driver", script)
        self.assertIn("$KernelSignatureIndex = $Lock.kernelSignatureIndex", script)
        self.assertIn("'windivert_signature.py'", script)

    def test_invalid_kernel_signature_policy_fails_closed(self):
        path = self.root.parent / "invalid-lock.json"
        for index in [None, True, "1", 1.5, -1, 8]:
            with self.subTest(index=index):
                lock = copy.deepcopy(self.lock)
                lock["kernelSignatureIndex"] = index
                path.write_text(json.dumps(lock))
                with mock.patch.object(vendor, "LOCK_PATH", path):
                    with self.assertRaisesRegex(vendor.VerificationError, "Kernel signature index"):
                        vendor.load_lock()

    def test_http_redirect_is_rejected(self):
        handler = vendor.HttpsOnlyRedirect()
        with self.assertRaisesRegex(vendor.VerificationError, "Non-HTTPS"):
            handler.redirect_request(None, None, 302, "", {}, "http://example.com/vendor.zip")


if __name__ == "__main__":
    unittest.main()
