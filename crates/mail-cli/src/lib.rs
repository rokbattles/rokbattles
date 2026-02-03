#![forbid(unsafe_code)]

//! Library helpers for the `mail-cli` binary.
//!
//! The CLI scans an input directory for mail binary buffers, decodes each buffer
//! into JSON using the `mail-decoder` crate, and writes JSON files alongside the
//! input data (or to a specified output directory).

use std::fs;
use std::path::{Path, PathBuf};

use mail_decoder::{
    DecodeError, LosslessArray, LosslessContainer, LosslessDocument, LosslessEncodeError,
    LosslessEntry, LosslessObject, LosslessValue, encode_lossless, lossless_to_json,
};
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

/// Summary of a rebuild run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildSummary {
    /// Number of lossless JSON files rebuilt into raw buffers.
    pub rebuilt_files: usize,
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
    /// Failed to parse lossless JSON input.
    LosslessJson {
        /// The serializer error.
        source: serde_json::Error,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// Lossless JSON payload did not match the expected schema.
    LosslessFormat {
        /// The format error.
        message: String,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// Failed to encode a lossless document back into bytes.
    LosslessEncode {
        /// The encoder error.
        source: LosslessEncodeError,
        /// Path associated with the lossless input.
        path: PathBuf,
    },
    /// The input path was not a file or directory.
    InvalidInputPath {
        /// The offending path.
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
            MailCliError::LosslessJson { source, path } => {
                write!(
                    f,
                    "lossless JSON parse failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::LosslessFormat { message, path } => {
                write!(
                    f,
                    "lossless JSON format error for {}: {message}",
                    path.display()
                )
            }
            MailCliError::LosslessEncode { source, path } => {
                write!(
                    f,
                    "lossless JSON encode failed for {}: {source}",
                    path.display()
                )
            }
            MailCliError::InvalidInputPath { path } => {
                write!(
                    f,
                    "input path is not a file or directory: {}",
                    path.display()
                )
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
            MailCliError::LosslessJson { source, .. } => Some(source),
            MailCliError::LosslessEncode { source, .. } => Some(source),
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

/// Rebuild lossless JSON files into raw mail buffers.
pub fn rebuild_lossless(config: &RebuildConfig) -> Result<RebuildSummary, MailCliError> {
    let input_files = collect_lossless_json_files(&config.input_path)?;
    if input_files.is_empty() {
        return Ok(RebuildSummary { rebuilt_files: 0 });
    }

    if config.mail_id.is_some() && input_files.len() != 1 {
        return Err(MailCliError::LosslessFormat {
            message: "--mail-id requires a single input file".to_string(),
            path: config.input_path.clone(),
        });
    }

    let output_dir = match &config.output_dir {
        Some(dir) => dir.clone(),
        None => {
            let metadata = fs::metadata(&config.input_path).map_err(|source| MailCliError::Io {
                source,
                path: config.input_path.clone(),
            })?;
            if metadata.is_file() {
                config
                    .input_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            } else {
                config.input_path.clone()
            }
        }
    };

    fs::create_dir_all(&output_dir).map_err(|source| MailCliError::Io {
        source,
        path: output_dir.clone(),
    })?;

    let mut rebuilt_files = 0;
    for input in input_files {
        let buffer = fs::read(&input).map_err(|source| MailCliError::Io {
            source,
            path: input.clone(),
        })?;
        let value: Value =
            serde_json::from_slice(&buffer).map_err(|source| MailCliError::LosslessJson {
                source,
                path: input.clone(),
            })?;
        let document =
            parse_lossless_document(&value).map_err(|message| MailCliError::LosslessFormat {
                message,
                path: input.clone(),
            })?;
        let mail_id = match &config.mail_id {
            Some(id) => id.clone(),
            None => extract_lossless_mail_id(&document.value).ok_or_else(|| {
                MailCliError::LosslessFormat {
                    message: "missing mail id in lossless JSON; supply --mail-id".to_string(),
                    path: input.clone(),
                }
            })?,
        };

        let output_path = output_dir.join(format!("Persistent.Mail.{mail_id}"));
        let encoded =
            encode_lossless(&document).map_err(|source| MailCliError::LosslessEncode {
                source,
                path: input.clone(),
            })?;
        fs::write(&output_path, encoded).map_err(|source| MailCliError::Io {
            source,
            path: output_path,
        })?;
        rebuilt_files += 1;
    }

    Ok(RebuildSummary { rebuilt_files })
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

fn collect_lossless_json_files(path: &Path) -> Result<Vec<PathBuf>, MailCliError> {
    let metadata = fs::metadata(path).map_err(|source| MailCliError::Io {
        source,
        path: path.to_path_buf(),
    })?;
    if metadata.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !metadata.is_dir() {
        return Err(MailCliError::InvalidInputPath {
            path: path.to_path_buf(),
        });
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(path).map_err(|source| MailCliError::Io {
        source,
        path: path.to_path_buf(),
    })? {
        let entry = entry.map_err(|source| MailCliError::Io {
            source,
            path: path.to_path_buf(),
        })?;
        let entry_path = entry.path();
        if entry_path.is_file() && is_json_file(&entry_path) {
            files.push(entry_path);
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
    let processed_input = match value {
        Value::Object(_) => Some(value),
        Value::Array(items) => match items.as_slice() {
            [item] if item.is_object() => Some(item),
            _ => None,
        },
        _ => None,
    };
    let Some(processed_input) = processed_input else {
        return Ok(());
    };

    // Only emit processed output for mail types with dedicated processors.
    let mail_type = processed_input.get("type").and_then(|value| value.as_str());
    let processed = match mail_type {
        Some("BarCanyonKillBoss") => Some(
            mail_processor_barcanyonkillboss::process_parallel(processed_input).map_err(
                |source| MailCliError::Process {
                    source,
                    path: input_path.to_path_buf(),
                },
            )?,
        ),
        Some("Battle") => Some(
            mail_processor_battle::process_parallel(processed_input).map_err(|source| {
                MailCliError::Process {
                    source,
                    path: input_path.to_path_buf(),
                }
            })?,
        ),
        Some("DuelBattle2") => Some(
            mail_processor_duelbattle2::process_parallel(processed_input).map_err(|source| {
                MailCliError::Process {
                    source,
                    path: input_path.to_path_buf(),
                }
            })?,
        ),
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

fn parse_lossless_document(value: &Value) -> Result<LosslessDocument, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "lossless JSON root must be an object".to_string())?;
    let preamble_hex = obj
        .get("preamble_hex")
        .and_then(Value::as_str)
        .ok_or_else(|| "lossless JSON missing preamble_hex".to_string())?;
    let preamble = decode_hex(preamble_hex)?;
    let value = obj
        .get("value")
        .ok_or_else(|| "lossless JSON missing value".to_string())?;
    let lossless_value = parse_lossless_value(value)?;
    Ok(LosslessDocument {
        preamble,
        value: lossless_value,
    })
}

fn parse_lossless_value(value: &Value) -> Result<LosslessValue, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "lossless value must be an object".to_string())?;
    let tag = obj
        .get("tag")
        .and_then(Value::as_str)
        .ok_or_else(|| "lossless value missing tag".to_string())?;

    match tag {
        "bool" => {
            let raw = obj
                .get("raw")
                .and_then(Value::as_u64)
                .ok_or_else(|| "lossless bool missing raw".to_string())?;
            let raw = u8::try_from(raw)
                .map_err(|_| "lossless bool raw must be in 0..=255".to_string())?;
            Ok(LosslessValue::Bool { value: raw })
        }
        "f32_le" => {
            let raw_hex = obj
                .get("raw_hex")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless f32 missing raw_hex".to_string())?;
            let bytes = decode_hex(raw_hex)?;
            if bytes.len() != 4 {
                return Err("lossless f32 raw_hex must be 4 bytes".to_string());
            }
            Ok(LosslessValue::F32 {
                raw: bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "lossless f32 raw_hex length invalid".to_string())?,
            })
        }
        "f64_be" => {
            let raw_hex = obj
                .get("raw_hex")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless f64 missing raw_hex".to_string())?;
            let bytes = decode_hex(raw_hex)?;
            if bytes.len() != 8 {
                return Err("lossless f64 raw_hex must be 8 bytes".to_string());
            }
            Ok(LosslessValue::F64 {
                raw: bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "lossless f64 raw_hex length invalid".to_string())?,
            })
        }
        "string" => {
            let value = obj
                .get("value")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless string missing value".to_string())?;
            Ok(LosslessValue::String {
                value: value.to_string(),
            })
        }
        "unknown" => {
            let raw = obj
                .get("raw")
                .and_then(Value::as_u64)
                .ok_or_else(|| "lossless unknown missing raw".to_string())?;
            let raw = u8::try_from(raw)
                .map_err(|_| "lossless unknown raw must be in 0..=255".to_string())?;
            Ok(LosslessValue::Unknown { tag: raw })
        }
        "container" => {
            let kind = obj
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless container missing kind".to_string())?;
            match kind {
                "object" => {
                    let entries = obj
                        .get("entries")
                        .and_then(Value::as_array)
                        .ok_or_else(|| "lossless object missing entries".to_string())?;
                    let mut parsed_entries = Vec::with_capacity(entries.len());
                    for entry in entries {
                        let entry_obj = entry
                            .as_object()
                            .ok_or_else(|| "lossless object entry must be an object".to_string())?;
                        let key = entry_obj
                            .get("key")
                            .and_then(Value::as_str)
                            .ok_or_else(|| "lossless object entry missing key".to_string())?;
                        let value = entry_obj
                            .get("value")
                            .ok_or_else(|| "lossless object entry missing value".to_string())?;
                        let value = parse_lossless_value(value)?;
                        parsed_entries.push(LosslessEntry {
                            key: key.to_string(),
                            value,
                        });
                    }
                    let terminator = obj
                        .get("terminator")
                        .and_then(Value::as_u64)
                        .map(|raw| {
                            u8::try_from(raw).map_err(|_| {
                                "lossless object terminator must be in 0..=255".to_string()
                            })
                        })
                        .transpose()?;
                    Ok(LosslessValue::Container(LosslessContainer::Object(
                        LosslessObject {
                            entries: parsed_entries,
                            terminator,
                        },
                    )))
                }
                "array" => {
                    let items = obj
                        .get("items")
                        .and_then(Value::as_array)
                        .ok_or_else(|| "lossless array missing items".to_string())?;
                    let mut parsed_items = Vec::with_capacity(items.len());
                    for item in items {
                        parsed_items.push(parse_lossless_value(item)?);
                    }
                    let terminator = obj
                        .get("terminator")
                        .and_then(Value::as_u64)
                        .map(|raw| {
                            u8::try_from(raw).map_err(|_| {
                                "lossless array terminator must be in 0..=255".to_string()
                            })
                        })
                        .transpose()?;
                    Ok(LosslessValue::Container(LosslessContainer::Array(
                        LosslessArray {
                            items: parsed_items,
                            terminator,
                        },
                    )))
                }
                _ => Err(format!("lossless container has unknown kind {kind}")),
            }
        }
        _ => Err(format!("lossless value has unknown tag {tag}")),
    }
}

fn decode_hex(input: &str) -> Result<Vec<u8>, String> {
    if !input.len().is_multiple_of(2) {
        return Err("hex string must have even length".to_string());
    }
    let mut bytes = Vec::with_capacity(input.len() / 2);
    let raw = input.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        let hi = hex_nibble(raw[i])?;
        let lo = hex_nibble(raw[i + 1])?;
        bytes.push((hi << 4) | lo);
        i += 2;
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!(
            "hex string contains invalid character {}",
            byte as char
        )),
    }
}

fn extract_lossless_mail_id(value: &LosslessValue) -> Option<String> {
    match value {
        LosslessValue::Container(LosslessContainer::Object(object)) => {
            for entry in &object.entries {
                if (entry.key.eq_ignore_ascii_case("id")
                    || entry.key.eq_ignore_ascii_case("mail_id"))
                    && let Some(id) = lossless_value_to_string(&entry.value)
                {
                    return Some(id);
                }
            }
            for entry in &object.entries {
                if let Some(id) = extract_lossless_mail_id(&entry.value) {
                    return Some(id);
                }
            }
            None
        }
        LosslessValue::Container(LosslessContainer::Array(array)) => {
            array.items.iter().find_map(extract_lossless_mail_id)
        }
        _ => None,
    }
}

fn lossless_value_to_string(value: &LosslessValue) -> Option<String> {
    match value {
        LosslessValue::String { value } => Some(value.clone()),
        LosslessValue::F64 { raw } => {
            let num = f64::from_be_bytes(*raw);
            if !num.is_finite() {
                return None;
            }
            let rounded = num.round();
            if (num - rounded).abs() > f64::EPSILON {
                return None;
            }
            if !(0.0..=9_007_199_254_740_992.0).contains(&rounded) {
                return None;
            }
            Some(format!("{rounded:.0}"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mail_decoder::{decode_lossless, lossless_to_json};
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
    fn rebuild_lossless_roundtrip_sample() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/rebuild/60719727166813248216.json");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = RebuildConfig {
            input_path: sample_path.clone(),
            output_dir: Some(output_dir.path().to_path_buf()),
            mail_id: None,
        };
        let summary = rebuild_lossless(&config).expect("rebuild lossless");
        assert_eq!(summary.rebuilt_files, 1);

        let rebuilt_path = output_dir
            .path()
            .join("Persistent.Mail.60719727166813248216");
        let rebuilt_bytes = fs::read(rebuilt_path).expect("read rebuilt bytes");
        let decoded = decode_lossless(&rebuilt_bytes).expect("decode rebuilt bytes");
        let roundtrip = lossless_to_json(&decoded);

        let original_json = fs::read_to_string(sample_path).expect("read sample json");
        let original_value: Value = serde_json::from_str(&original_json).expect("parse sample");
        assert_eq!(roundtrip, original_value);
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
    fn write_processed_json_handles_singleton_array() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/Battle/Persistent.Mail.33830971176980291131.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");
        assert!(matches!(value, Value::Array(_)));

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("33830971176980291131"));
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
