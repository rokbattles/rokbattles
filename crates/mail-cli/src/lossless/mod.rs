use std::{ffi::OsStr, fs, path::Path};

use mail_decoder::encode_lossless;
use serde_json::Value;

use crate::{
    MailCliError, MigrateConfig, MigrateSummary, RebuildConfig, RebuildSummary,
    fs_utils::write_json_value,
};

mod files;
mod hex;
mod mail_id;
mod parse;

use files::{collect_lossless_json_files, resolve_output_dir_for};
use mail_id::{extract_lossless_mail_id, validate_mail_id};
use parse::parse_lossless_document;

/// Rebuild lossless JSON files back into raw mail buffers.
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

    let output_dir = resolve_output_dir_for(&config.input_path, &config.output_dir)?;
    fs::create_dir_all(&output_dir)
        .map_err(|source| MailCliError::Io { source, path: output_dir.clone() })?;

    let mut rebuilt_files = 0;
    for input in input_files {
        let buffer =
            fs::read(&input).map_err(|source| MailCliError::Io { source, path: input.clone() })?;
        let value: Value = serde_json::from_slice(&buffer)
            .map_err(|source| MailCliError::LosslessJson { source, path: input.clone() })?;
        let document = parse_lossless_document(&value)
            .map_err(|message| MailCliError::LosslessFormat { message, path: input.clone() })?;
        let mail_id = match &config.mail_id {
            Some(id) => id.clone(),
            None => extract_lossless_mail_id(&document.value).ok_or_else(|| {
                MailCliError::LosslessFormat {
                    message: "missing mail id in lossless JSON; supply --mail-id".to_string(),
                    path: input.clone(),
                }
            })?,
        };
        validate_mail_id(&mail_id)
            .map_err(|message| MailCliError::LosslessFormat { message, path: input.clone() })?;

        let output_path = output_dir.join(format!("Persistent.Mail.{mail_id}"));
        let encoded = encode_lossless(&document)
            .map_err(|source| MailCliError::LosslessEncode { source, path: input.clone() })?;
        fs::write(&output_path, encoded)
            .map_err(|source| MailCliError::Io { source, path: output_path })?;
        rebuilt_files += 1;
    }

    Ok(RebuildSummary { rebuilt_files })
}

/// Convert lossless JSON files into v2 JSON through `binary-cursor`.
pub fn migrate_lossless(config: &MigrateConfig) -> Result<MigrateSummary, MailCliError> {
    let input_files = collect_lossless_json_files(&config.input_path)?;
    if input_files.is_empty() {
        return Ok(MigrateSummary { migrated_files: 0 });
    }

    let output_dir = resolve_output_dir_for(&config.input_path, &config.output_dir)?;
    fs::create_dir_all(&output_dir)
        .map_err(|source| MailCliError::Io { source, path: output_dir.clone() })?;

    let mut migrated_files = 0;
    for input in input_files {
        let (value, mail_id) = load_lossless_json(&input)?;
        let document = parse_lossless_document(&value)
            .map_err(|message| MailCliError::LosslessFormat { message, path: input.clone() })?;
        let mail_id = extract_lossless_mail_id(&document.value).or(mail_id).ok_or_else(|| {
            MailCliError::LosslessFormat {
                message: "missing mail id in lossless JSON or file name".to_string(),
                path: input.clone(),
            }
        })?;
        validate_mail_id(&mail_id)
            .map_err(|message| MailCliError::LosslessFormat { message, path: input.clone() })?;

        // Rebuild the exact bytes first so migrated files and new binary files
        // go through the same decoder code path.
        let encoded = encode_lossless(&document)
            .map_err(|source| MailCliError::LosslessEncode { source, path: input.clone() })?;
        let decoded = binary_cursor::decode(&encoded)
            .map_err(|source| MailCliError::BinaryDecode { source, path: input.clone() })?;
        let output_path = output_dir.join(format!("Persistent.Mail.{mail_id}-v2.json"));
        write_json_value(&output_path, &decoded, config.pretty)?;
        migrated_files += 1;
    }

    Ok(MigrateSummary { migrated_files })
}

fn load_lossless_json(input: &Path) -> Result<(Value, Option<String>), MailCliError> {
    let buffer =
        fs::read(input).map_err(|source| MailCliError::Io { source, path: input.to_path_buf() })?;
    let value: Value = serde_json::from_slice(&buffer)
        .map_err(|source| MailCliError::LosslessJson { source, path: input.to_path_buf() })?;
    Ok((value, mail_id_from_file_name(input)))
}

fn mail_id_from_file_name(path: &Path) -> Option<String> {
    let stem = path.file_stem().and_then(OsStr::to_str)?;
    let mail_id = stem.strip_prefix("Persistent.Mail.").unwrap_or(stem);
    Some(mail_id.strip_suffix("-v2").unwrap_or(mail_id).to_string())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use mail_decoder::{decode_lossless, lossless_to_json};
    use serde_json::Value;

    use super::*;

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

        let rebuilt_path = output_dir.path().join("Persistent.Mail.60719727166813248216");
        let rebuilt_bytes = fs::read(rebuilt_path).expect("read rebuilt bytes");
        let decoded = decode_lossless(&rebuilt_bytes).expect("decode rebuilt bytes");
        let roundtrip = lossless_to_json(&decoded);

        let original_json = fs::read_to_string(sample_path).expect("read sample json");
        let original_value: Value = serde_json::from_str(&original_json).expect("parse sample");
        assert_eq!(roundtrip, original_value);
    }

    #[test]
    fn migrate_lossless_writes_binary_cursor_v2_json() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/rebuild/60719727166813248216.json");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = MigrateConfig {
            input_path: sample_path,
            output_dir: Some(output_dir.path().to_path_buf()),
            pretty: true,
        };
        let summary = migrate_lossless(&config).expect("migrate lossless");
        assert_eq!(summary.migrated_files, 1);

        let migrated_path = output_dir.path().join("Persistent.Mail.60719727166813248216-v2.json");
        let json = fs::read_to_string(migrated_path).expect("read migrated json");
        let value: Value = serde_json::from_str(&json).expect("parse migrated json");
        assert!(value.is_object());
    }

    #[test]
    fn migrate_lossless_matches_binary_cursor_decode_of_existing_binary_sample() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lossless_path = repo_root.join("samples/rebuild/60719727166813248216.json");
        let binary_path = repo_root.join("samples/Battle/Persistent.Mail.60719727166813248216");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = MigrateConfig {
            input_path: lossless_path,
            output_dir: Some(output_dir.path().to_path_buf()),
            pretty: false,
        };
        let summary = migrate_lossless(&config).expect("migrate lossless");
        assert_eq!(summary.migrated_files, 1);

        let migrated_path = output_dir.path().join("Persistent.Mail.60719727166813248216-v2.json");
        let migrated_json = fs::read_to_string(migrated_path).expect("read migrated json");
        let migrated: Value = serde_json::from_str(&migrated_json).expect("parse migrated json");

        let binary = fs::read(binary_path).expect("read binary sample");
        let decoded = binary_cursor::decode(&binary).expect("decode binary sample");

        assert_eq!(migrated, decoded);
    }

    #[test]
    fn migrate_lossless_drops_positional_numeric_keys_but_keeps_semantic_numeric_keys() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/rebuild/10200933177249262113.json");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = MigrateConfig {
            input_path: sample_path,
            output_dir: Some(output_dir.path().to_path_buf()),
            pretty: false,
        };
        migrate_lossless(&config).expect("migrate lossless");

        let migrated_path = output_dir.path().join("Persistent.Mail.10200933177249262113-v2.json");
        let migrated_json = fs::read_to_string(migrated_path).expect("read migrated json");
        let migrated: Value = serde_json::from_str(&migrated_json).expect("parse migrated json");

        assert!(migrated.pointer("/body/content/SelfChar/HSS").is_some_and(Value::is_array));
        assert!(migrated.pointer("/body/content/SelfChar/Samples").is_none());
        assert!(migrated.pointer("/body/content/Samples").is_some_and(Value::is_array));
        assert!(migrated.pointer("/body/content/SelfChar/HWBs").is_some_and(Value::is_object));
    }

    #[test]
    fn migrate_lossless_uses_file_stem_mail_id_when_document_has_no_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input_path = temp.path().join("123.json");
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
        let config = MigrateConfig {
            input_path,
            output_dir: Some(output_dir.path().to_path_buf()),
            pretty: false,
        };

        let summary = migrate_lossless(&config).unwrap();
        assert_eq!(summary.migrated_files, 1);
        let output_path = output_dir.path().join("Persistent.Mail.123-v2.json");
        let migrated_json = fs::read_to_string(output_path).expect("read migrated json");
        let migrated: Value = serde_json::from_str(&migrated_json).expect("parse migrated json");
        assert_eq!(migrated, serde_json::json!({}));
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
                assert_eq!(message, "missing mail id in lossless JSON; supply --mail-id");
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn rebuild_lossless_rejects_cli_mail_id_with_path_separator() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/rebuild/60719727166813248216.json");
        let output_dir = tempfile::tempdir().expect("output dir");

        let config = RebuildConfig {
            input_path: sample_path,
            output_dir: Some(output_dir.path().to_path_buf()),
            mail_id: Some("../../outside".to_string()),
        };

        let err = rebuild_lossless(&config).unwrap_err();
        match err {
            MailCliError::LosslessFormat { message, .. } => {
                assert_eq!(message, "mail id cannot contain path separators");
            }
            _ => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn rebuild_lossless_uses_parent_directory_when_single_file_has_no_output_dir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let input_path = temp.path().join("single.json");
        let json = r#"
{
  "preamble_hex": "",
  "value": {
    "tag": "container",
    "kind": "object",
    "entries": [
      {
        "key": "id",
        "value": {
          "tag": "string",
          "value": "123"
        }
      }
    ]
  }
}
"#;
        fs::write(&input_path, json).expect("write lossless json");

        let config =
            RebuildConfig { input_path: input_path.clone(), output_dir: None, mail_id: None };
        let summary = rebuild_lossless(&config).unwrap();
        assert_eq!(summary.rebuilt_files, 1);
        assert!(temp.path().join("Persistent.Mail.123").exists());
    }
}
