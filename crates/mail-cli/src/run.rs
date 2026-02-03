use std::fs;
use std::path::{Path, PathBuf};

use mail_decoder::lossless_to_json;
use serde_json::Value;

use crate::fs_utils::is_json_file;
use crate::{Config, MailCliError, RunSummary};

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

pub(crate) fn collect_input_files(dir: &Path) -> Result<Vec<PathBuf>, MailCliError> {
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

pub(crate) fn output_path(output_dir: &Path, input_path: &Path) -> Result<PathBuf, MailCliError> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MailCliError::MissingFileName {
            path: input_path.to_path_buf(),
        })?;
    Ok(output_dir.join(format!("{file_name}.json")))
}

/// Build the output path for processed JSON files.
pub(crate) fn processed_output_path(
    output_dir: &Path,
    input_path: &Path,
) -> Result<PathBuf, MailCliError> {
    let file_name = input_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MailCliError::MissingFileName {
            path: input_path.to_path_buf(),
        })?;
    Ok(output_dir.join(format!("{file_name}-processed.json")))
}

/// Write processed JSON if a known processor exists for the mail type.
pub(crate) fn write_processed_json(
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

    #[test]
    fn write_processed_json_writes_duelbattle2_metadata() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input = temp.path().join("sample.mail");
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/DuelBattle2/Persistent.Mail.4197312176618249531.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        write_processed_json(temp.path(), &input, &value, true).unwrap();
        let output = processed_output_path(temp.path(), &input).unwrap();
        let output_json = fs::read_to_string(output).expect("read processed");
        let parsed: Value = serde_json::from_str(&output_json).expect("parse processed");
        assert_eq!(parsed["metadata"]["mail_id"], json!("4197312176618249531"));
    }
}
