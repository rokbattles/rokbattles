"""Read-only verification of an exact embedded signature with Windows driver policy."""

import ctypes
import os
from pathlib import Path
import sys
import uuid

import windivert_vendor as vendor

# Fixed-width Windows ABI types, including when structure tests run on Linux.
DWORD = ctypes.c_uint32
POINTER = ctypes.c_void_p


class Guid(ctypes.Structure):
    _fields_ = [("data1", DWORD), ("data2", ctypes.c_uint16),
                ("data3", ctypes.c_uint16), ("data4", ctypes.c_ubyte * 8)]


class FileInfo(ctypes.Structure):
    _fields_ = [("size", DWORD), ("path", ctypes.c_wchar_p),
                ("handle", POINTER), ("subject", POINTER)]


class SignatureSettings(ctypes.Structure):
    _fields_ = [("size", DWORD), ("index", DWORD), ("flags", DWORD),
                ("secondary_count", DWORD), ("verified_index", DWORD),
                ("crypto_policy", POINTER)]


class TrustData(ctypes.Structure):
    _fields_ = [("size", DWORD), ("policy_data", POINTER), ("sip_data", POINTER),
                ("ui", DWORD), ("revocation", DWORD), ("choice", DWORD),
                ("file_info", POINTER), ("state_action", DWORD), ("state", POINTER),
                ("url", POINTER), ("flags", DWORD), ("ui_context", DWORD),
                ("signatures", POINTER)]


def check_result(status, requested_index, verified_index):
    # WinVerifyTrust returns a LONG, not an HRESULT. Only zero is success.
    if status != 0:
        raise vendor.VerificationError(f"Windows driver policy rejected signature: 0x{status & 0xffffffff:08x}")
    if verified_index != requested_index:
        raise vendor.VerificationError("Windows did not verify the requested driver signature")


def verify_kernel_signature(path, index):
    if type(index) is not int or not 0 <= index <= 7:
        raise vendor.VerificationError("Kernel signature index must be an integer between 0 and 7")
    if os.name != "nt":
        raise vendor.VerificationError("Windows is required for driver-policy verification")

    # Load only OS libraries from System32. The vendor DLL/SYS is never executed.
    kernel = ctypes.WinDLL("kernel32.dll", use_last_error=True, winmode=0x800)
    trust = ctypes.WinDLL("wintrust.dll", use_last_error=True, winmode=0x800)
    kernel.CreateFileW.argtypes = [ctypes.c_wchar_p, DWORD, DWORD, POINTER, DWORD, DWORD, POINTER]
    kernel.CreateFileW.restype = POINTER
    kernel.CloseHandle.argtypes = [POINTER]
    kernel.CloseHandle.restype = ctypes.c_int32
    trust.WinVerifyTrust.argtypes = [POINTER, ctypes.POINTER(Guid), ctypes.POINTER(TrustData)]
    trust.WinVerifyTrust.restype = ctypes.c_int32

    absolute = str(Path(path).resolve(strict=True))
    # GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING: deny modification/deletion
    # while Windows reads the original image. No catalog installation or lookup.
    handle = kernel.CreateFileW(absolute, 0x80000000, 1, None, 3, 0x80, None)
    if handle == POINTER(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())

    file_info = FileInfo(ctypes.sizeof(FileInfo), absolute, handle, None)
    settings = SignatureSettings(ctypes.sizeof(SignatureSettings), index, 1, 0, 0xffffffff, None)
    data = TrustData()
    data.size = ctypes.sizeof(TrustData)
    data.ui = 2  # WTD_UI_NONE
    data.revocation = 1  # WTD_REVOKE_WHOLECHAIN
    data.choice = 1  # WTD_CHOICE_FILE (embedded signature only)
    data.file_info = ctypes.addressof(file_info)
    data.state_action = 1  # WTD_STATEACTION_VERIFY
    data.flags = 0x80 | 0x2000  # chain revocation except root; disable MD2/MD4
    data.signatures = ctypes.addressof(settings)  # WSS_VERIFY_SPECIFIC
    # DRIVER_ACTION_VERIFY is Microsoft's driver/WHQL policy, not generic /pa.
    # https://learn.microsoft.com/windows/win32/api/wintrust/nf-wintrust-winverifytrust
    action = Guid.from_buffer_copy(uuid.UUID("f750e6c3-38ee-11d1-85e5-00c04fc295ee").bytes_le)
    try:
        try:
            status = trust.WinVerifyTrust(POINTER(-1), ctypes.byref(action), ctypes.byref(data))
            check_result(status, index, settings.verified_index)
        finally:
            # Every VERIFY call must be paired with CLOSE, including failures.
            data.state_action = 2
            trust.WinVerifyTrust(POINTER(-1), ctypes.byref(action), ctypes.byref(data))
    finally:
        kernel.CloseHandle(handle)


def main():
    lock = vendor.load_lock()
    try:
        vendor.verify(lock)
        verify_kernel_signature(vendor.OUTPUT / "WinDivert64.sys", lock["kernelSignatureIndex"])
        vendor.verify(lock)
    except (OSError, ValueError) as error:
        print(f"WinDivert driver-policy verification failed: {error}", file=sys.stderr)
        return 1
    print(f"Windows driver policy verified exact embedded signature {lock['kernelSignatureIndex']}; no driver loaded")
    return 0


if __name__ == "__main__":
    sys.exit(main())
