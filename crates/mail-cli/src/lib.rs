#![forbid(unsafe_code)]

//! Library helpers for the `mail-cli` binary.
//!
//! The CLI scans an input directory for mail binary buffers, decodes each buffer
//! into JSON using the `mail-decoder` crate, and writes JSON files alongside the
//! input data (or to a specified output directory).

use std::fs;
use std::path::{Path, PathBuf};

use mail_decoder::{DecodeError, lossless_to_json};
use mail_processor_sdk::ProcessError;
use serde_json::Value;

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

/// Errors that can occur while decoding a directory.
#[derive(Debug)]
pub enum MailCliError {
    /// The input path was not a directory.
    InvalidInputDir {
        /// The offending path.
        path: PathBuf,
    },
    /// Failed to read or write from the filesystem.
    Io {
        /// The underlying I/O error.
        source: std::io::Error,
        /// Path associated with the I/O failure.
        path: PathBuf,
    },
    /// Failed to decode a mail buffer.
    Decode {
        /// The decoder error.
        source: DecodeError,
        /// Path to the buffer being decoded.
        path: PathBuf,
    },
    /// Failed to serialize the decoded JSON value.
    Json {
        /// The serializer error.
        source: serde_json::Error,
        /// Path associated with the output.
        path: PathBuf,
    },
    /// Failed to process decoded mail JSON.
    Process {
        /// The processor error.
        source: ProcessError,
        /// Path associated with the processing failure.
        path: PathBuf,
    },
    /// The input file did not have a usable file name.
    MissingFileName {
        /// The offending path.
        path: PathBuf,
    },
}

impl std::fmt::Display for MailCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailCliError::InvalidInputDir { path } => {
                write!(f, "input path is not a directory: {}", path.display())
            }
            MailCliError::Io { source, path } => {
                write!(f, "I/O error for {}: {source}", path.display())
            }
            MailCliError::Decode { source, path } => {
                write!(f, "decode failed for {}: {source}", path.display())
            }
            MailCliError::Json { source, path } => {
                write!(
                    f,
                    "JSON serialization failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::Process { source, path } => {
                write!(f, "processing failed for {}: {source}", path.display())
            }
            MailCliError::MissingFileName { path } => {
                write!(f, "missing file name for path: {}", path.display())
            }
        }
    }
}

impl std::error::Error for MailCliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MailCliError::Io { source, .. } => Some(source),
            MailCliError::Decode { source, .. } => Some(source),
            MailCliError::Json { source, .. } => Some(source),
            MailCliError::Process { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Decode every mail buffer in the input directory into JSON files.
pub fn run(config: &Config) -> Result<RunSummary, MailCliError> {
    let metadata = fs::metadata(&config.input_dir).map_err(|source| MailCliError::Io {
        source,
        path: config.input_dir.clone(),
    })?;

    if !metadata.is_dir() {
        return Err(MailCliError::InvalidInputDir {
            path: config.input_dir.clone(),
        });
    }

    fs::create_dir_all(&config.output_dir).map_err(|source| MailCliError::Io {
        source,
        path: config.output_dir.clone(),
    })?;

    let input_files = collect_input_files(&config.input_dir)?;
    let mut decoded_files = 0;

    for input in input_files {
        decode_file(&input, &config.output_dir, config.pretty, config.lossless)?;
        decoded_files += 1;
    }

    Ok(RunSummary { decoded_files })
}

fn collect_input_files(dir: &Path) -> Result<Vec<PathBuf>, MailCliError> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(|source| MailCliError::Io {
        source,
        path: dir.to_path_buf(),
    })? {
        let entry = entry.map_err(|source| MailCliError::Io {
            source,
            path: dir.to_path_buf(),
        })?;
        let path = entry.path();
        if path.is_file() && !is_json_file(&path) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn is_json_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

fn decode_file(
    input: &Path,
    output_dir: &Path,
    pretty: bool,
    lossless: bool,
) -> Result<(), MailCliError> {
    let buffer = fs::read(input).map_err(|source| MailCliError::Io {
        source,
        path: input.to_path_buf(),
    })?;
    if lossless {
        let document =
            mail_decoder::decode_lossless(&buffer).map_err(|source| MailCliError::Decode {
                source,
                path: input.to_path_buf(),
            })?;
        let value = lossless_to_json(&document);
        write_json(output_dir, input, &value, pretty)?;
        return Ok(());
    }

    let value = mail_decoder::decode(&buffer).map_err(|source| MailCliError::Decode {
        source,
        path: input.to_path_buf(),
    })?;
    write_json(output_dir, input, &value, pretty)?;
    write_processed_json(output_dir, input, &value, pretty)?;
    Ok(())
}

fn write_json(
    output_dir: &Path,
    input_path: &Path,
    value: &Value,
    pretty: bool,
) -> Result<(), MailCliError> {
    let output_path = output_path(output_dir, input_path)?;
    let json = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|source| MailCliError::Json {
        source,
        path: output_path.clone(),
    })?;

    fs::write(&output_path, json).map_err(|source| MailCliError::Io {
        source,
        path: output_path,
    })?;
    Ok(())
}

fn output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf, MailCliError> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MailCliError::MissingFileName {
            path: input_path.to_path_buf(),
        })?;
    Ok(output_dir.join(format!("{file_name}.json")))
}

/// Build the output path for processed JSON files.
fn processed_output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf, MailCliError> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MailCliError::MissingFileName {
            path: input_path.to_path_buf(),
        })?;
    Ok(output_dir.join(format!("{file_name}-processed.json")))
}

/// Write processed JSON if a known processor exists for the mail type.
fn write_processed_json(
    output_dir: &Path,
    input_path: &Path,
    value: &Value,
    pretty: bool,
) -> Result<(), MailCliError> {
    // Only emit processed output for mail types with dedicated processors.
    let mail_type = value.get("type").and_then(|value| value.as_str());
    let processed = match mail_type {
        Some("BarCanyonKillBoss") => Some(
            mail_processor_barcanyonkillboss::process_parallel(value).map_err(|source| {
                MailCliError::Process {
                    source,
                    path: input_path.to_path_buf(),
                }
            })?,
        ),
        Some("Battle") => Some(mail_processor_battle::process_parallel(value).map_err(
            |source| MailCliError::Process {
                source,
                path: input_path.to_path_buf(),
            },
        )?),
        Some("DuelBattle2") => Some(mail_processor_duelbattle2::process_parallel(value).map_err(
            |source| MailCliError::Process {
                source,
                path: input_path.to_path_buf(),
            },
        )?),
        _ => None,
    };

    let processed = match processed {
        Some(processed) => processed,
        None => return Ok(()),
    };

    let output_path = processed_output_path(output_dir, input_path)?;
    let json = if pretty {
        serde_json::to_string_pretty(&processed)
    } else {
        serde_json::to_string(&processed)
    }
    .map_err(|source| MailCliError::Json {
        source,
        path: output_path.clone(),
    })?;

    fs::write(&output_path, json).map_err(|source| MailCliError::Io {
        source,
        path: output_path,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;

    fn write_bytes(path: &Path, bytes: &[u8]) {
        let mut file = File::create(path).expect("create file");
        file.write_all(bytes).expect("write bytes");
    }

    #[test]
    fn output_path_appends_json_extension() {
        let output = PathBuf::from("/tmp/out");
        let input = PathBuf::from("/tmp/in/sample.bin");
        let result = output_path(&output, &input).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/out/sample.bin.json"));
    }

    #[test]
    fn processed_output_path_appends_suffix() {
        let output = PathBuf::from("/tmp/out");
        let input = PathBuf::from("/tmp/in/sample.bin");
        let result = processed_output_path(&output, &input).unwrap();
        assert_eq!(result, PathBuf::from("/tmp/out/sample.bin-processed.json"));
    }

    #[test]
    fn collect_input_files_skips_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        write_bytes(&dir.join("a.bin"), &[0x01, 1]);
        write_bytes(&dir.join("b.json"), b"{}");
        write_bytes(&dir.join("c"), &[0x01, 0]);

        let files = collect_input_files(dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("a.bin")));
        assert!(files.iter().any(|path| path.ends_with("c")));
    }

    #[test]
    fn run_decodes_and_writes_pretty_json() {
        let input_dir = tempfile::tempdir().expect("input dir");
        let output_dir = tempfile::tempdir().expect("output dir");
        let input_path = input_dir.path().join("sample.mail");

        let buffer = vec![0x05, 0x04, 0x01, 0, 0, 0, b'a', 0x01, 1, 0xff];
        write_bytes(&input_path, &buffer);

        let config = Config {
            input_dir: input_dir.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            pretty: true,
            lossless: false,
        };
        let summary = run(&config).unwrap();
        assert_eq!(summary.decoded_files, 1);

        let output_path = output_dir.path().join("sample.mail.json");
        let json = fs::read_to_string(output_path).expect("read output");
        assert_eq!(json, "{\n  \"a\": true\n}");
    }

    #[test]
    fn run_decodes_lossless_json() {
        let input_dir = tempfile::tempdir().expect("input dir");
        let output_dir = tempfile::tempdir().expect("output dir");
        let input_path = input_dir.path().join("sample.mail");

        let mut buffer = vec![0xff, 0x00];
        buffer.extend_from_slice(&[0x05, 0xff]);
        write_bytes(&input_path, &buffer);

        let config = Config {
            input_dir: input_dir.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            pretty: true,
            lossless: true,
        };
        let summary = run(&config).unwrap();
        assert_eq!(summary.decoded_files, 1);

        let output_path = output_dir.path().join("sample.mail.json");
        let json = fs::read_to_string(output_path).expect("read output");
        let value: Value = serde_json::from_str(&json).expect("parse output");
        assert_eq!(value["preamble_hex"], Value::String("ff00".to_string()));
    }

    #[test]
    fn run_rejects_non_directory_input() {
        let temp = tempfile::tempdir().expect("temp dir");
        let file_path = temp.path().join("file.bin");
        write_bytes(&file_path, &[0x01, 1]);

        let config = Config {
            input_dir: file_path,
            output_dir: temp.path().join("out"),
            pretty: true,
            lossless: false,
        };
        let err = run(&config).unwrap_err();
        assert!(matches!(err, MailCliError::InvalidInputDir { .. }));
    }

    #[test]
    fn write_processed_json_skips_unknown_type() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let value = json!({ "type": "Unknown" });

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        assert!(!output.exists());
    }

    #[test]
    fn write_processed_json_writes_battle_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.1002579517552941234.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("1002579517552941234"));
    }

    #[test]
    fn write_processed_json_writes_barcanyonkillboss_metadata_and_npc() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/BarCanyonKillBoss/Persistent.Mail.21162669176948646831.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("21162669176948646831"));
        assert_eq!(parsed["npc"]["type"], json!(102000055));
        assert_eq!(parsed["npc"]["level"], json!(25));
    }
}
