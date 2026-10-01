//! Darwin extended ACLs are independent of Unix mode bits. Deny-only entries
//! are harmless (and normal on home directories); any extra grant is rejected.

use std::{
    ffi::c_void,
    fs::File,
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    ptr,
};

use crate::Error;

// Darwin SDK sys/acl.h: opaque ACL/entry handles, ACL_TYPE_EXTENDED=0x100,
// ACL_FIRST_ENTRY=0, ACL_NEXT_ENTRY=-1, ACL_EXTENDED_DENY=2.
unsafe extern "C" {
    fn filesec_init() -> *mut c_void;
    fn filesec_free(security: *mut c_void);
    fn filesec_query_property(
        security: *mut c_void,
        property: libc::c_int,
        present: *mut libc::c_int,
    ) -> libc::c_int;
    fn filesec_get_property(
        security: *mut c_void,
        property: libc::c_int,
        value: *mut c_void,
    ) -> libc::c_int;
    #[cfg_attr(target_arch = "x86_64", link_name = "fstatx_np$INODE64")]
    fn fstatx_np(fd: libc::c_int, metadata: *mut libc::stat, security: *mut c_void) -> libc::c_int;
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
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    self::file(&file)
}

struct FileSecurity(*mut c_void);

impl Drop for FileSecurity {
    fn drop(&mut self) {
        // SAFETY: owned allocation from filesec_init.
        unsafe { filesec_free(self.0) };
    }
}

pub(super) fn file(file: &File) -> Result<(), Error> {
    // SAFETY: no arguments; returns an owned opaque descriptor.
    let raw = unsafe { filesec_init() };
    if raw.is_null() {
        return Err(Error::Security);
    }
    let security = FileSecurity(raw);
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live FD, correctly sized stat output and initialized filesec handle.
    if unsafe { fstatx_np(file.as_raw_fd(), metadata.as_mut_ptr(), security.0) } != 0 {
        return Err(Error::Security);
    }
    let mut present = 0;
    // SAFETY: valid filesec snapshot and integer output; FILESEC_ACL is 5.
    if unsafe { filesec_query_property(security.0, 5, &mut present) } != 0 {
        return Err(Error::Security);
    }
    if present == 0 {
        return Ok(());
    }
    let mut acl: *mut c_void = ptr::null_mut();
    // SAFETY: present ACL property returns a separately owned ACL pointer.
    if unsafe { filesec_get_property(security.0, 5, (&mut acl as *mut *mut c_void).cast()) } != 0 {
        return Err(Error::Security);
    }
    verify(acl)
}
