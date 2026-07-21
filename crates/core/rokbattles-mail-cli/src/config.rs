use std::path::PathBuf;

/// Settings for decoding a directory of mail buffers.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory that contains the input mail buffers.
    pub input_dir: PathBuf,
    /// Directory where decoded JSON files should be written.
    pub output_dir: PathBuf,
    /// Whether to pretty-print the JSON output.
    pub pretty: bool,
}

/// Result summary for a decode run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Number of files that were decoded and written.
    pub decoded_files: usize,
}
