use std::path::PathBuf;

/// Configuration for decoding a directory of mail buffers.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory containing input mail buffers.
    pub input_dir: PathBuf,
    /// Directory where JSON output files will be written.
    pub output_dir: PathBuf,
    /// Whether to pretty-print JSON output.
    pub pretty: bool,
    /// Whether to decode using the lossless representation.
    pub lossless: bool,
}

/// Summary of a decode run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Number of files decoded and written.
    pub decoded_files: usize,
}

/// Configuration for rebuilding lossless JSON into raw mail buffers.
#[derive(Debug, Clone)]
pub struct RebuildConfig {
    /// File or directory containing lossless JSON documents.
    pub input_path: PathBuf,
    /// Directory where rebuilt mail buffers will be written. Defaults to the input directory
    /// (or the input file parent directory).
    pub output_dir: Option<PathBuf>,
    /// Mail id override for single-file inputs.
    pub mail_id: Option<String>,
}

/// Summary of a rebuild run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildSummary {
    /// Number of lossless JSON files rebuilt into raw buffers.
    pub rebuilt_files: usize,
}
