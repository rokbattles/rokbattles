use std::fs;

use mail_decoder::encode_lossless;
use serde_json::Value;

use crate::{MailCliError, RebuildConfig, RebuildSummary};

mod files;
mod hex;
mod mail_id;
mod parse;

use files::{collect_lossless_json_files, resolve_output_dir};
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

    let output_dir = resolve_output_dir(config)?;
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
