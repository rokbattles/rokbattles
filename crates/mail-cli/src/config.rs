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
    /// Whether to decode through the lossless representation.
    pub lossless: bool,
    /// Decode binary files with `binary-cursor` instead of the old decoder.
    pub binary_cursor: bool,
}

/// Result summary for a decode run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Number of files that were decoded and written.
    pub decoded_files: usize,
}

/// Settings for rebuilding lossless JSON into raw mail buffers.
#[derive(Debug, Clone)]
pub struct RebuildConfig {
    /// File or directory that contains the lossless JSON documents.
    pub input_path: PathBuf,
    /// Directory where rebuilt mail buffers should be written.
    /// Defaults to the input directory, or the input file's parent directory.
    pub output_dir: Option<PathBuf>,
    /// Mail ID override for single-file inputs.
    pub mail_id: Option<String>,
}

/// Result summary for a rebuild run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildSummary {
    /// Number of lossless JSON files rebuilt into raw buffers.
    pub rebuilt_files: usize,
}

/// Settings for converting stored lossless JSON into v2 decoded JSON.
#[derive(Debug, Clone)]
pub struct MigrateConfig {
    /// File or directory that contains the lossless JSON documents.
    pub input_path: PathBuf,
    /// Directory where v2 JSON files should be written.
    /// Defaults to the input directory, or the input file's parent directory.
    pub output_dir: Option<PathBuf>,
    /// Whether to pretty-print the v2 JSON output.
    pub pretty: bool,
}

/// Result summary for a lossless-to-v2 run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrateSummary {
    /// Number of lossless JSON files migrated into v2 JSON.
    pub migrated_files: usize,
}
