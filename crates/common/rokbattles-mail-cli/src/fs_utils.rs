//! Filesystem helpers for deterministic selection and JSON output.
//!
//! Path-aware errors distinguish source access from destination serialization and
//! write failures. Output helpers expect the destination directory to exist.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::Value;

use crate::MailCliError;

/// Tests the final extension for `json`, ignoring ASCII case.
pub(crate) fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

/// Requires a directory; metadata lookup failures remain I/O errors.
pub(crate) fn ensure_directory(path: &Path) -> Result<(), MailCliError> {
    let metadata = fs::metadata(path)
        .map_err(|source| MailCliError::Io { source, path: path.to_path_buf() })?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(MailCliError::InvalidInputDir { path: path.to_path_buf() })
    }
}

/// Collects matching immediate files and sorts their paths.
///
/// `Path::is_file` follows symlinks and excludes paths whose metadata cannot
/// be read. Directory iteration errors are returned.
pub(crate) fn collect_sorted_files<F>(
    dir: &Path,
    mut predicate: F,
) -> Result<Vec<PathBuf>, MailCliError>
where
    F: FnMut(&Path) -> bool,
{
    let mut files = Vec::new();
    for entry in
        fs::read_dir(dir).map_err(|source| MailCliError::Io { source, path: dir.to_path_buf() })?
    {
        let entry = entry.map_err(|source| MailCliError::Io { source, path: dir.to_path_buf() })?;
        let path = entry.path();
        if path.is_file() && predicate(&path) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

/// Serializes to memory, then creates or overwrites the destination file.
///
/// Serialization errors leave the destination untouched. Writes are direct,
/// so an I/O failure may leave an existing file truncated or partially written.
pub(crate) fn write_json_file<T: Serialize>(
    output_path: &Path,
    value: &T,
    pretty: bool,
) -> Result<(), MailCliError> {
    let json = if pretty { serde_json::to_vec_pretty(value) } else { serde_json::to_vec(value) }
        .map_err(|source| MailCliError::Json { source, path: output_path.to_path_buf() })?;

    fs::write(output_path, json)
        .map_err(|source| MailCliError::Io { source, path: output_path.to_path_buf() })
}

/// Writes a decoded JSON value using the same format as processor output.
pub(crate) fn write_json_value(
    output_path: &Path,
    value: &Value,
    pretty: bool,
) -> Result<(), MailCliError> {
    write_json_file(output_path, value, pretty)
}

/// Builds a destination name from the entire UTF-8 input filename.
///
/// Appends the suffix and extension rather than replacing an existing extension.
/// Missing or non-UTF-8 filenames return `MissingFileName`.
pub(crate) fn output_path_with_suffix(
    output_dir: &Path,
    input_path: &Path,
    suffix: &str,
    extension: &str,
) -> Result<PathBuf, MailCliError> {
    let file_name = input_path
        .file_name()
        .filter(|name| *name != OsStr::new(""))
        .and_then(|name| name.to_str())
        .ok_or_else(|| MailCliError::MissingFileName { path: input_path.to_path_buf() })?;
    Ok(output_dir.join(format!("{file_name}{suffix}.{extension}")))
}
