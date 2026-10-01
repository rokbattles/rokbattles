"""Driver trust checks; native cases only read files and never activate a driver."""

import ctypes
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import windivert_signature as signature
import windivert_vendor as vendor


class SignatureResultTests(unittest.TestCase):
    def test_only_zero_status_and_exact_index_are_accepted(self):
        signature.check_result(0, 1, 1)
        for status in [1, 2, -2146762487, 0x800B0109]:
            with self.subTest(status=status):
                with self.assertRaises(vendor.VerificationError):
                    signature.check_result(status, 1, 1)
        for actual in [0, 2, 0xffffffff]:
            with self.subTest(index=actual):
                with self.assertRaises(vendor.VerificationError):
                    signature.check_result(0, 1, actual)

    def test_invalid_index_is_rejected_before_os_calls(self):
        for index in [None, True, "1", 1.5, -1, 8]:
            with self.subTest(index=index):
                with self.assertRaisesRegex(vendor.VerificationError, "Kernel signature index"):
                    signature.verify_kernel_signature("does-not-exist.sys", index)

    def test_windows_abi_layout(self):
        self.assertEqual(ctypes.sizeof(signature.Guid), 16)
        if ctypes.sizeof(ctypes.c_void_p) == 8:
            self.assertEqual(ctypes.sizeof(signature.FileInfo), 32)
            self.assertEqual(ctypes.sizeof(signature.SignatureSettings), 32)
            self.assertEqual(ctypes.sizeof(signature.TrustData), 88)
            self.assertEqual(signature.TrustData.signatures.offset, 80)

    def test_native_state_and_file_are_closed_on_success_and_rejection(self):
        path = Path(__file__).resolve()
        for status, verified in [(0, 1), (1, 1), (0, 0)]:
            with self.subTest(status=status, verified=verified):
                kernel = mock.Mock()
                kernel.CreateFileW.return_value = 123
                trust = mock.Mock()
                actions = []

                def verify(_window, _action, pointer):
                    data = ctypes.cast(pointer, ctypes.POINTER(signature.TrustData)).contents
                    actions.append(data.state_action)
                    if data.state_action == 2:
                        return 0
                    self.assertEqual(data.revocation, 1)
                    self.assertEqual(data.flags, 0x80 | 0x2000)
                    self.assertEqual(data.choice, 1)
                    settings = ctypes.cast(data.signatures, ctypes.POINTER(signature.SignatureSettings)).contents
                    self.assertEqual(settings.flags, 1)
                    self.assertEqual(settings.index, 1)
                    self.assertEqual(settings.verified_index, 0xffffffff)
                    settings.verified_index = verified
                    return status

                trust.WinVerifyTrust.side_effect = verify
                with mock.patch.object(signature.os, "name", "nt"), \
                        mock.patch.object(signature, "Path", return_value=path), \
                        mock.patch.object(signature.ctypes, "WinDLL", create=True,
                                          side_effect=[kernel, trust]):
                    if status == 0 and verified == 1:
                        signature.verify_kernel_signature(path, 1)
                    else:
                        with self.assertRaises(vendor.VerificationError):
                            signature.verify_kernel_signature(path, 1)
                self.assertEqual(actions, [1, 2])
                kernel.CloseHandle.assert_called_once_with(123)


@unittest.skipUnless(os.name == "nt" and os.environ.get("WINDIVERT_NATIVE_TRUST_TESTS") == "1",
                     "Requires Windows and explicitly staged pinned test resources")
class NativeSignatureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.lock = vendor.load_lock()
        vendor.verify(cls.lock)
        cls.driver = vendor.OUTPUT / "WinDivert64.sys"

    def test_exact_microsoft_driver_signature_is_trusted(self):
        signature.verify_kernel_signature(self.driver, self.lock["kernelSignatureIndex"])
        vendor.verify(self.lock)

    def test_primary_vendor_signature_does_not_satisfy_driver_policy(self):
        with self.assertRaises(vendor.VerificationError):
            signature.verify_kernel_signature(self.driver, 0)

    def test_missing_signature_index_is_rejected(self):
        with self.assertRaises(vendor.VerificationError):
            signature.verify_kernel_signature(self.driver, 7)

    def test_unsigned_library_is_rejected(self):
        with self.assertRaises(vendor.VerificationError):
            signature.verify_kernel_signature(vendor.OUTPUT / "WinDivert.dll", 0)

    def test_tampered_copy_is_rejected(self):
        # Mutate a disposable test copy's executable section, never the staged
        # vendor file or its certificate table. Nothing in either file is run.
        data = bytearray(self.driver.read_bytes())
        data[4096] ^= 1
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tampered.sys"
            path.write_bytes(data)
            with self.assertRaises(vendor.VerificationError):
                signature.verify_kernel_signature(path, self.lock["kernelSignatureIndex"])
        vendor.verify(self.lock)


if __name__ == "__main__":
    unittest.main()
