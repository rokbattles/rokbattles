use std::path::Path;

use libloading::Library;

use crate::Error;

/// The caller establishes trust in the library, its dependencies and ABI.
/// Absolute paths avoid searching PATH, the working directory, or loader paths
/// for the primary library. Canonicalization is not a trust or signature check.
pub(crate) unsafe fn load(path: &Path) -> Result<Library, Error> {
    if !path.is_absolute() {
        return Err(Error::InvalidInput("native library path must be absolute"));
    }

    let path = path
        .canonicalize()
        .map_err(|error| Error::Library { path: path.to_owned(), detail: error.to_string() })?;

    #[cfg(windows)]
    // SAFETY: caller vouches for the binary and its ABI. The primary path is
    // absolute; dependencies may only come from its trusted directory (Npcap's
    // Packet.dll lives beside wpcap.dll) or System32, never cwd or PATH.
    let library = unsafe {
        libloading::os::windows::Library::load_with_flags(
            &path,
            libloading::os::windows::LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR
                | libloading::os::windows::LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
        .map(Library::from)
    };

    #[cfg(not(windows))]
    // SAFETY: caller vouches for the binary, dependencies and native initializers.
    let library = unsafe { Library::new(&path) };

    library.map_err(|error| Error::Library { path, detail: error.to_string() })
}

/// Every copied pointer stays private to an owner retaining `library`.
pub(crate) unsafe fn symbol<T: Copy>(
    library: &Library,
    name: &'static std::ffi::CStr,
) -> Result<T, Error> {
    // SAFETY: the caller supplies the documented native signature for this symbol.
    let symbol = unsafe { library.get::<T>(name.to_bytes_with_nul()) };

    symbol.map(|value| *value).map_err(|error| Error::Symbol {
        name: name.to_str().unwrap_or("non-UTF8 symbol"),
        detail: error.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_paths_without_loading() {
        // SAFETY: input validation returns before any loader invocation.
        let error = unsafe { load(Path::new("untrusted-library")) }.expect_err("relative path");

        assert!(matches!(error, Error::InvalidInput(_)));
    }

    #[test]
    fn missing_file_is_a_recoverable_error() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml").join("not-a-directory");
        // SAFETY: the nonexistent path fails canonicalization without loading code.
        let error = unsafe { load(&path) }.expect_err("missing file");

        assert!(matches!(error, Error::Library { .. }));
    }

    #[test]
    fn missing_symbol_is_a_recoverable_error() {
        // Query only the already-running test executable; never load a capture
        // library, execute library initializers, or open a native capture.
        #[cfg(unix)]
        let library = Library::from(libloading::os::unix::Library::this());
        #[cfg(windows)]
        let library =
            Library::from(libloading::os::windows::Library::this().expect("current executable"));

        // SAFETY: this deliberately absent symbol is never invoked. Its name
        // cannot be provided by any of this crate's native test mocks.
        let result = unsafe {
            symbol::<unsafe extern "C" fn()>(&library, c"rokbattles_missing_symbol_867655b1")
        };

        assert!(matches!(result, Err(Error::Symbol { .. })));
    }
}
