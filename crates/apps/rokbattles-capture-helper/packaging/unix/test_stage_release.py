"""Release orchestration uses synthetic binaries and mocked Apple tools only."""
import ast
import hashlib
import json
from pathlib import Path
import plistlib
import tempfile
import unittest

import stage_release
from test_stage_package import binary

ENVIRONMENT = {"APPLE_TEAM_ID": "AB12345678", "APPLE_SIGNING_IDENTITY": "fixture-identity", "APPLE_ID": "fixture-account", "APPLE_PASSWORD": "fixture-secret"}


class ReleaseTests(unittest.TestCase):
    def fixtures(self, root, target):
        helper = root / "helper"
        agent = root / "agent"
        installer = root / "maintain.py"
        helper.write_bytes(binary(target))
        agent.write_bytes(binary(target))
        installer.write_bytes(b'"""Synthetic inert installer."""\n' + stage_release.PIN_MARKER + b'\n')
        return helper, agent, installer

    def test_linux_assets_are_uid_neutral_and_checksum_every_deliverable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = "x86_64-unknown-linux-gnu"
            helper, agent, installer = self.fixtures(root, target)
            def no_commands(*_args):
                self.fail("Linux packaging must not execute tools or binaries")
            files = stage_release.prepare(target, helper, agent, installer, root / "out", {}, no_commands)
            self.assertEqual(len(files), 3)
            checksums = files[-1].read_text()
            for path in files[:-1]:
                self.assertIn(hashlib.sha256(path.read_bytes()).hexdigest() + "  " + path.name, checksums)
            self.assertNotIn("uid", checksums)
            tree = ast.parse(files[1].read_bytes())
            assignment = next(node for node in tree.body if isinstance(node, ast.Assign))
            pins = ast.literal_eval(assignment.value)
            self.assertEqual(pins["archiveSha256"], hashlib.sha256(files[0].read_bytes()).hexdigest())
            self.assertEqual(pins["target"], target)
            self.assertIsNone(pins["appleTeamId"])
            self.assertNotIn(stage_release.PIN_MARKER, files[1].read_bytes())

    def test_mac_signs_both_exact_identifiers_and_requires_accepted_notarization(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = "aarch64-apple-darwin"
            helper, agent, installer = self.fixtures(root, target)
            commands = []
            def mocked(label, arguments, timeout=120):
                commands.append((label, arguments, timeout))
                if label == "Apple notarization":
                    return b'{"status":"Accepted","id":"fixture-id","uploadUrl":"not-for-publication"}'
                return b""
            files = stage_release.prepare(target, helper, agent, installer, root / "out", ENVIRONMENT, mocked)
            sign = [args for label, args, _ in commands if label == "Hardened code signing"]
            self.assertEqual(len(sign), 2)
            verification = [args for label, args, _ in commands if label == "Signature and team verification"]
            self.assertTrue(all(args[args.index("--test-requirement") + 1].startswith("=anchor apple generic") for args in verification))
            self.assertTrue(all("runtime" in args for args in sign))
            self.assertEqual({args[args.index("--identifier") + 1] for args in sign}, set(stage_release.IDENTIFIERS.values()))
            receipt = json.loads(next(path for path in files if path.name.endswith(".notarization.json")).read_text())
            self.assertNotIn("uploadUrl", receipt)
            self.assertNotIn("fixture-secret", json.dumps(receipt))

    def test_mac_fails_before_payload_for_injection_entitlements_or_notary_rejection(self):
        for failure in ["entitlements", "notary"]:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = "x86_64-apple-darwin"
                helper, agent, installer = self.fixtures(root, target)
                def mocked(label, _arguments, _timeout=120):
                    if failure == "entitlements" and label == "Entitlement inspection":
                        return plistlib.dumps({"com.apple.security.cs.disable-executable-page-protection": True})
                    if label == "Apple notarization":
                        return b'{"status":"Invalid","id":"fixture-id"}'
                    return b""
                with self.assertRaises(stage_release.ReleaseError):
                    stage_release.prepare(target, helper, agent, installer, root / "out", ENVIRONMENT, mocked)
                self.assertFalse(list((root / "out" / "assets").glob("*.tar.gz")))

    def test_installer_template_cannot_omit_or_duplicate_embedded_pins(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / "fixture.tar.gz"
            archive.write_bytes(b"fixture")
            for source in [b"print('no pins')", stage_release.PIN_MARKER + b"\n" + stage_release.PIN_MARKER]:
                with self.assertRaises(stage_release.ReleaseError):
                    stage_release.render_installer(source, "x86_64-unknown-linux-gnu", archive, None)

    def test_team_identifier_cannot_inject_a_code_requirement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = {**ENVIRONMENT, "APPLE_TEAM_ID": 'AB12345678" or true'}
            with self.assertRaises(stage_release.ReleaseError):
                stage_release.sign_and_notarize([], root, environment, lambda *_args: self.fail("must reject before tools"))


if __name__ == "__main__":
    unittest.main()
