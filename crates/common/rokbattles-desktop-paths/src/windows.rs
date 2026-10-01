//! Owner/SYSTEM-only state ACLs. This code never loads a driver or elevates.

use std::{
    ffi::c_void,
    fs::{self, File},
    mem::{size_of, size_of_val},
    os::windows::{
        ffi::OsStrExt,
        fs::MetadataExt,
        io::{FromRawHandle, OwnedHandle},
    },
    path::{Component, Path, Prefix},
    ptr,
};

use windows_sys::Win32::{
    Foundation::{ERROR_ALREADY_EXISTS, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetNamedSecurityInfoW, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetSecurityDescriptorControl,
        GetTokenInformation, IsValidSid, IsWellKnownSid, OWNER_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER, TokenUser, WinBuiltinAdministratorsSid, WinCreatorOwnerRightsSid,
        WinLocalSystemSid,
    },
    Storage::FileSystem::{
        CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_ALWAYS,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

use crate::Error;

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // SAFETY: these allocations are returned by documented LocalAlloc-based Win32 APIs.
        unsafe { LocalFree(self.0) };
    }
}

struct User {
    data: Vec<usize>,
}

impl User {
    fn current() -> Result<Self, Error> {
        let mut token = ptr::null_mut();
        // SAFETY: current-process pseudo handle is valid and output points to a HANDLE.
        let process = unsafe { GetCurrentProcess() };
        // SAFETY: live pseudo handle and a writable token output.
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
            return Err(Error::Security);
        }
        // SAFETY: OpenProcessToken returned an owned, non-null token handle.
        let _token = unsafe { OwnedHandle::from_raw_handle(token) };
        let mut data = vec![0usize; 32];
        let mut required = 0u32;
        // SAFETY: usize storage is suitably aligned for TOKEN_USER; its fixed 256-byte
        // capacity on supported targets exceeds TOKEN_USER plus the maximum SID.
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                data.as_mut_ptr().cast(),
                size_of_val(data.as_slice()) as u32,
                &mut required,
            )
        } == 0
            || required as usize > size_of_val(data.as_slice())
        {
            return Err(Error::Security);
        }
        Ok(Self { data })
    }

    fn sid(&self) -> PSID {
        // SAFETY: successful TokenUser query initialized this aligned heap buffer;
        // the SID pointer remains valid because the Vec allocation never moves.
        unsafe { (*self.data.as_ptr().cast::<TOKEN_USER>()).User.Sid }
    }

    fn descriptor(&self) -> Result<LocalAllocation, Error> {
        let mut sid_text = ptr::null_mut();
        // SAFETY: sid is from the successful TokenUser query; output is writable.
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut sid_text) } == 0 {
            return Err(Error::Security);
        }
        let _sid_text = LocalAllocation(sid_text.cast());
        let mut length = 0usize;
        // SAFETY: ConvertSidToStringSidW returns a valid NUL-terminated string.
        while unsafe { *sid_text.wrapping_add(length) } != 0 {
            length += 1;
        }
        // SAFETY: length was measured within that NUL-terminated allocation.
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text, length) })
            .map_err(|_error| Error::Security)?;
        let sddl = format!("O:{sid}D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid})");
        let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // SAFETY: string is terminated; descriptor output is writable. Revision 1 is SDDL_REVISION_1.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                1,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(Error::Security);
        }
        Ok(LocalAllocation(descriptor))
    }
}

fn wide(path: &Path) -> Result<Vec<u16>, Error> {
    let bytes: Vec<u16> = path.as_os_str().encode_wide().collect();
    if bytes.contains(&0) {
        return Err(Error::Untrusted);
    }
    Ok(bytes.into_iter().chain(Some(0)).collect())
}

fn trusted_sid(sid: PSID, user: &User, private: bool) -> bool {
    // SAFETY: callers pass OS-returned/bounds-checked SIDs with a live descriptor.
    let owner = unsafe { EqualSid(sid, user.sid()) != 0 };
    // SAFETY: same validated SID.
    let system = unsafe { IsWellKnownSid(sid, WinLocalSystemSid) != 0 };
    if private {
        // SAFETY: Owner Rights applies only after the object owner was verified.
        owner || system || unsafe { IsWellKnownSid(sid, WinCreatorOwnerRightsSid) != 0 }
    } else {
        // SAFETY: Administrators may mutate ancestor directories, never the private state DACL.
        owner || system || unsafe { IsWellKnownSid(sid, WinBuiltinAdministratorsSid) != 0 }
    }
}

fn security(path: &Path, user: &User, private: bool, protected: bool) -> Result<(), Error> {
    let name = wide(path)?;
    let mut owner = ptr::null_mut();
    let mut acl: *mut ACL = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    // SAFETY: terminated file path and correctly typed output pointers.
    let status = unsafe {
        GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(Error::Security);
    }
    if descriptor.is_null() {
        return Err(Error::Security);
    }
    let _descriptor = LocalAllocation(descriptor);
    let mut control = 0u16;
    let mut revision = 0u32;
    // SAFETY: descriptor is OS-returned and remains live; outputs are correctly typed.
    if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
        return Err(Error::Security);
    }
    if protected && control & SE_DACL_PROTECTED == 0 {
        return Err(Error::Untrusted);
    }
    if acl.is_null() || owner.is_null() {
        return Err(Error::Untrusted);
    }
    // SAFETY: owner belongs to this live security descriptor.
    if private && unsafe { EqualSid(owner, user.sid()) } == 0 {
        return Err(Error::Untrusted);
    }
    if !private && !trusted_sid(owner, user, false) {
        return Err(Error::Untrusted);
    }

    // SAFETY: GetNamedSecurityInfoW returned a valid ACL header.
    let header = unsafe { ptr::read_unaligned(acl) };
    let (count, size) = (header.AceCount, usize::from(header.AclSize));
    let start = acl as usize;
    let end = start.checked_add(size).ok_or(Error::Untrusted)?;
    for index in 0..u32::from(count) {
        let mut raw = ptr::null_mut();
        // SAFETY: index is bounded by the returned ACL AceCount.
        if unsafe { GetAce(acl, index, &mut raw) } == 0 || raw.is_null() {
            return Err(Error::Untrusted);
        }
        let address = raw as usize;
        if address < start + size_of::<ACL>()
            || address.checked_add(size_of::<ACE_HEADER>()).is_none_or(|last| last > end)
        {
            return Err(Error::Untrusted);
        }
        // SAFETY: checked that the header lies inside the ACL; unaligned read avoids alignment assumptions.
        let header = unsafe { ptr::read_unaligned(raw.cast::<ACE_HEADER>()) };
        let size = usize::from(header.AceSize);
        if size < size_of::<ACE_HEADER>() || address.checked_add(size).is_none_or(|last| last > end)
        {
            return Err(Error::Untrusted);
        }
        if header.AceType == 1 {
            continue;
        } // Deny ACEs cannot grant access.
        if !private && header.AceFlags & 0x08 != 0 {
            continue;
        } // Ancestor inherit-only entry.
        if header.AceType != 0 || size < size_of::<ACCESS_ALLOWED_ACE>() + 4 {
            return Err(Error::Untrusted);
        }
        // SAFETY: full allow ACE and SID header fit in the checked ACE extent.
        let ace = unsafe { ptr::read_unaligned(raw.cast::<ACCESS_ALLOWED_ACE>()) };
        let sid_offset = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);
        // SAFETY: two SID header bytes fit because the eight-byte SID header was checked above.
        let subauthorities = unsafe { *raw.cast::<u8>().wrapping_add(sid_offset + 1) } as usize;
        if subauthorities > 15 || sid_offset + 8 + 4 * subauthorities > size {
            return Err(Error::Untrusted);
        }
        // SAFETY: full SID extent fits within the ACE allocation.
        let sid = unsafe { raw.cast::<u8>().add(sid_offset) }.cast();
        // SAFETY: the complete SID extent was checked above before validation.
        if unsafe { IsValidSid(sid) } == 0 {
            return Err(Error::Untrusted);
        }
        const MUTATE_PARENT: u32 = 0x10000000
            | 0x40000000
            | 0x00010000
            | 0x00040000
            | 0x00080000
            | 0x2
            | 0x10
            | 0x40
            | 0x100;
        if (private || ace.Mask & MUTATE_PARENT != 0) && !trusted_sid(sid, user, private) {
            return Err(Error::Untrusted);
        }
    }
    Ok(())
}

pub(super) fn directory(path: &Path) -> Result<(), Error> {
    if !matches!(path.components().next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
    {
        return Err(Error::Untrusted);
    }
    let user = User::current()?;
    let mut missing = Vec::new();
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(metadata) => {
                if !metadata.is_dir()
                    || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
                {
                    return Err(Error::Untrusted);
                }
                security(parent, &user, parent == path, parent == path)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing.push(parent),
            Err(error) => return Err(error.into()),
        }
    }
    let descriptor = user.descriptor()?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    for parent in missing.into_iter().rev() {
        let name = wide(parent)?;
        // SAFETY: terminated path and live private security descriptor.
        if unsafe { CreateDirectoryW(name.as_ptr(), &attributes) } == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32) {
                return Err(error.into());
            }
        }
        let metadata = fs::symlink_metadata(parent)?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(Error::Untrusted);
        }
        security(parent, &user, true, true)?;
    }
    security(path, &user, true, true)
}

pub(super) fn existing_file(path: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                return Err(Error::Untrusted);
            }
            security(path, &User::current()?, true, false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn file(path: &Path) -> Result<File, Error> {
    existing_file(path)?;
    let user = User::current()?;
    let descriptor = user.descriptor()?;
    let name = wide(path)?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // SAFETY: file path and private descriptor are valid; OPEN_REPARSE_POINT prevents following a swapped link.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: CreateFileW returned an owned file handle.
    let file = unsafe { File::from_raw_handle(handle) };
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(Error::Untrusted);
    }
    security(path, &user, true, false)?;
    Ok(file)
}
