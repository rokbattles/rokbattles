use std::{
    fs::{self, File},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
};

use crate::Error;

fn uid() -> u32 {
    // SAFETY: geteuid takes no arguments and has no memory preconditions.
    unsafe { libc::geteuid() }
}

fn ancestors(path: &Path) -> Result<(), Error> {
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || !metadata.is_dir()
                    || (metadata.uid() != 0 && metadata.uid() != uid())
                {
                    return Err(Error::Untrusted);
                }
                let writable_by_others = metadata.mode() & 0o022 != 0;
                let protected_sticky = metadata.uid() == 0 && metadata.mode() & 0o1000 != 0;
                if writable_by_others && !protected_sticky {
                    return Err(Error::Untrusted);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn directory(path: &Path) -> Result<(), Error> {
    ancestors(path)?;
    fs::DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    ancestors(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != uid()
        || metadata.mode() & 0o077 != 0
    {
        return Err(Error::Untrusted);
    }
    Ok(())
}

pub(super) fn existing_file(path: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.uid() != uid()
                || metadata.mode() & 0o077 != 0
                || metadata.nlink() != 1
            {
                return Err(Error::Untrusted);
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn file(path: &Path) -> Result<File, Error> {
    existing_file(path)?;
    let file =
        crate::options().mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC).open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != uid()
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(Error::Untrusted);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn creates_private_state_and_rejects_links_or_shared_modes() {
        let parent = tempfile::tempdir().expect("tempdir");
        let state = parent.path().join("state");
        directory(&state).expect("private directory");
        let file = crate::file(&state, "worker.lock").expect("private file");
        assert_eq!(file.metadata().expect("metadata").mode() & 0o777, 0o600);
        drop(file);
        symlink(state.join("worker.lock"), state.join("bad")).expect("link");
        assert!(matches!(crate::file(&state, "bad"), Err(Error::Untrusted)));
        fs::set_permissions(&state, fs::Permissions::from_mode(0o755)).expect("test mode");
        assert!(matches!(directory(&state), Err(Error::Untrusted)));
    }

    #[test]
    fn rejects_linked_ancestor_and_hard_linked_database() {
        let parent = tempfile::tempdir().expect("tempdir");
        let state = parent.path().join("state");
        directory(&state).expect("private directory");
        symlink(&state, parent.path().join("linked")).expect("link");
        assert!(matches!(directory(&parent.path().join("linked/child")), Err(Error::Untrusted)));
        crate::file(&state, "database").expect("file");
        fs::hard_link(state.join("database"), state.join("other")).expect("hardlink");
        assert!(matches!(existing_file(&state.join("database")), Err(Error::Untrusted)));
    }
}
