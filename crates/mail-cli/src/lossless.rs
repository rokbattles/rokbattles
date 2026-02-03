use std::fs;
use std::path::{Path, PathBuf};

use mail_decoder::{
    LosslessArray, LosslessContainer, LosslessDocument, LosslessEntry, LosslessObject,
    LosslessValue, encode_lossless,
};
use serde_json::Value;

use crate::fs_utils::is_json_file;
use crate::{MailCliError, RebuildConfig, RebuildSummary};

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
    fn rebuild_lossless_rejects_mail_id_with_multiple_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input_dir = temp.path();
        fs::File::create(input_dir.join("a.json")).expect("create a.json");
        fs::File::create(input_dir.join("b.json")).expect("create b.json");

        let config = RebuildConfig {
            input_path: input_dir.to_path_buf(),
            output_dir: None,
            mail_id: Some("123".to_string()),
        };
        let err = rebuild_lossless(&config).unwrap_err();
        match err {
            MailCliError::LosslessFormat { message, .. } => {
                assert_eq!(message, "--mail-id requires a single input file");
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn rebuild_lossless_requires_mail_id_or_embedded_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input_path = temp.path().join("missing-id.json");
        let json = r#"
{
  "preamble_hex": "",
  "value": {
    "tag": "container",
    "kind": "object",
    "entries": []
  }
}
"#;
        fs::write(&input_path, json).expect("write lossless json");

        let output_dir = tempfile::tempdir().expect("output dir");
        let config = RebuildConfig {
            input_path: input_path.clone(),
            output_dir: Some(output_dir.path().to_path_buf()),
            mail_id: None,
        };
        let err = rebuild_lossless(&config).unwrap_err();
        match err {
            MailCliError::LosslessFormat { message, .. } => {
                assert_eq!(
                    message,
                    "missing mail id in lossless JSON; supply --mail-id"
                );
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn collect_lossless_json_files_only_includes_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path();
        fs::File::create(dir.join("a.json")).expect("create a.json");
        fs::File::create(dir.join("b.txt")).expect("create b.txt");
        fs::File::create(dir.join("c.JSON")).expect("create c.JSON");

        let files = collect_lossless_json_files(dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("a.json")));
        assert!(files.iter().any(|path| path.ends_with("c.JSON")));
    }
}
