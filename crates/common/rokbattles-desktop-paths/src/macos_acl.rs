//! Darwin extended ACLs are independent of Unix mode bits. Deny-only entries
//! are harmless (and normal on home directories); any extra grant is rejected.

use std::{
    ffi::{CString, c_void},
    fs::File,
    os::unix::{ffi::OsStrExt, io::AsRawFd},
    path::Path,
    ptr,
};

use crate::Error;

// Darwin SDK sys/acl.h: opaque ACL/entry handles, ACL_TYPE_EXTENDED=0x100,
// ACL_FIRST_ENTRY=0, ACL_NEXT_ENTRY=-1, ACL_EXTENDED_DENY=2.
unsafe extern "C" {
    fn acl_get_link_np(path: *const libc::c_char, kind: libc::c_int) -> *mut c_void;
    fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut c_void;
    fn acl_get_entry(acl: *mut c_void, index: libc::c_int, entry: *mut *mut c_void) -> libc::c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut libc::c_int) -> libc::c_int;
    fn acl_free(acl: *mut c_void) -> libc::c_int;
}

struct Acl(*mut c_void);

impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: this is an owned allocation returned by an ACL getter.
        unsafe { acl_free(self.0) };
    }
}

fn verify(raw: *mut c_void) -> Result<(), Error> {
    if raw.is_null() {
        return Err(Error::Security);
    }
    let acl = Acl(raw);
    let mut index = 0;
    for _ in 0..=128 {
        let mut entry = ptr::null_mut();
        // SAFETY: live OS-returned ACL, documented iterator value and valid output.
        if unsafe { acl_get_entry(acl.0, index, &mut entry) } != 0 {
            // Darwin returns -1/EINVAL at end, unlike Linux's 0/1 convention.
            return if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL) {
                Ok(())
            } else {
                Err(Error::Security)
            };
        }
        let mut tag = 0;
        // SAFETY: successful iterator produced an entry owned by the live ACL.
        if entry.is_null() || unsafe { acl_get_tag_type(entry, &mut tag) } != 0 || tag != 2 {
            return Err(Error::Untrusted);
        }
        index = -1;
    }
    Err(Error::Untrusted)
}

pub(super) fn path(path: &Path) -> Result<(), Error> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_error| Error::Untrusted)?;
    // SAFETY: NUL-terminated path; no-follow getter; ownership passes to verify.
    verify(unsafe { acl_get_link_np(path.as_ptr(), 0x100) })
}

pub(super) fn file(file: &File) -> Result<(), Error> {
    // SAFETY: borrowed live file descriptor; ownership of returned ACL passes to verify.
    verify(unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) })
}
