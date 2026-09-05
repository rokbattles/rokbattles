//! Settings and completion counts shared by the binary and library callers.

use std::path::PathBuf;

/// Settings for decoding files and writing any recognized processor output.
///
/// Library callers supply both directories explicitly. The binary resolves its
/// optional output directory before constructing this value.
#[derive(Debug, Clone)]
pub struct Config {
    /// Existing directory containing persistent mail files.
    ///
    /// Only immediate files are considered; JSON files are skipped.
    pub input_dir: PathBuf,
    /// Destination for decoded and processed JSON, created if necessary.
    ///
    /// May be the input directory. Existing outputs with matching names are overwritten.
    pub output_dir: PathBuf,
    /// Whether both output representations use indented rather than compact JSON.
    pub pretty: bool,
}

/// Completion summary returned after every selected input succeeds.
///
/// An empty directory, or one containing only skipped entries, yields zero.
/// Failed runs return an error without a partial summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Number of input files completed, including files with no recognized category.
    ///
    /// Counts each input once, regardless of whether it produced one or two outputs.
    pub decoded_files: usize,
}
