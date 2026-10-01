"""Synthetic packaging checks; never build/run a native helper or installer."""
import base64
import gzip
import hashlib
import json
import pathlib
import re
import tempfile
import unittest
from unittest.mock import patch

import stage_maintenance as stage


class MaintenancePackagingTests(unittest.TestCase):
    def test_both_architectures_pin_apps_and_use_per_machine_hooks(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            desktop = root / 'desktop'
            (desktop / 'windows').mkdir(parents=True)
            (desktop / 'windows/bootstrap.ps1').write_text((stage.DESKTOP / 'windows/bootstrap.ps1').read_text())
            built = root / 'built'
            built.mkdir()
            for name in ('rokbattles-capture-helper.exe', 'rokbattles-desktop-agent.exe', 'rokbattles-capture-maintenance.exe'):
                (built / name).write_bytes(b'synthetic fixture: ' + name.encode())
            with patch.object(stage, 'DESKTOP', desktop):
                for target in stage.TARGETS:
                    stage.generate(built, target)
                    overlay = json.loads((desktop / f'tauri.capture-{target}.conf.json').read_text())
                    self.assertEqual(overlay['bundle']['windows']['nsis']['installMode'], 'perMachine')
                    self.assertNotIn('windivert', json.dumps(overlay).lower())
                    self.assertNotIn('npcap', json.dumps(overlay).lower())
                    staged = desktop / 'binaries' / target
                    pins = json.loads((staged / 'payload.json').read_text())
                    self.assertEqual(pins['rokbattles-capture-helper.exe'], hashlib.sha256((built / 'rokbattles-capture-helper.exe').read_bytes()).hexdigest())
                    generated = (staged / 'bootstrap.nsh').read_text()
                    for body in re.findall(r'!macro ROK_APPEND_BOOTSTRAP_\w+\n(.*?)!macroend', generated, re.S):
                        chunks = re.findall(r'w "([A-Za-z0-9+/=]+)"', body)
                        self.assertTrue(chunks)
                        self.assertTrue(all(len(chunk) <= 512 for chunk in chunks))
                        self.assertTrue(all(len(line) < 1024 for line in body.splitlines()))
                        encoded = ''.join(chunks)
                        self.assertLessEqual(len(encoded), 14000)
                        self.assertLessEqual(2 * (1024 + 200 + len(encoded) + 1), 32768)
                        loader = base64.b64decode(encoded).decode('utf-16-le')
                        payload = loader.split("'")[1]
                        script = gzip.decompress(base64.b64decode(payload)).decode()
                        self.assertIn('Assert-Protected $ProgramFiles', script)
                        self.assertNotIn(str(root), script)
    def test_rejects_unknown_target(self):
        with self.assertRaises(ValueError):
            stage.generate(pathlib.Path('/unused'), 'x86-pc-windows-msvc')
    def test_hooks_keep_old_session_until_commit_and_finalize_pin_before_unblock(self):
        hooks = (stage.DESKTOP / 'windows/capture-hooks.nsh').read_text()
        self.assertLess(hooks.index('!insertmacro ROK_START_SESSION', hooks.index('!macro NSIS_HOOK_PREINSTALL')), hooks.index('File /oname=rokbattles-capture-maintenance.next.exe'))
        self.assertIn('MoveFileExW', hooks)
        self.assertNotIn('$PLUGINSDIR', hooks)
        self.assertIn('Var RokInstallerMutex', hooks)
        self.assertLess(hooks.index('!insertmacro ROK_LOCK_INSTALLER'), hooks.index('\"${ROK_MAINTENANCE}\" session'))
        self.assertIn('validate-installer-lock', hooks)
        self.assertNotIn('CloseHandle(p $RokInstallerMutex)', hooks)
        self.assertNotIn('ReleaseMutex', hooks)
        self.assertNotIn('taskkill', hooks.lower())
        self.assertIn('i 0x08000400, p $RokEnvironmentBuffer, w "$SYSDIR\\WindowsPowerShell\\v1.0"', hooks)
        self.assertIn('"PATH=$SYSDIR"', hooks)
        self.assertIn('"PSModulePath=$SYSDIR\\WindowsPowerShell\\v1.0\\Modules"', hooks)
        entries = re.findall(r'!insertmacro ROK_ENV_ENTRY "(.*?)"', hooks)
        self.assertEqual([entry.split('=')[0] for entry in entries], ['COMSPEC', 'PATH', 'PSModulePath', 'SystemRoot', 'WINDIR'])


if __name__ == '__main__':
    unittest.main()
