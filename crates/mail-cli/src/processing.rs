use anyhow::{Context, Result};
use mail_helper::EmailType;
use serde_json::Value;
use std::path::Path;

use crate::io::{self, OutputPaths};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum InputFormat {
    Binary,
    Json,
}

pub(crate) struct DecodedMail {
    pub(crate) id: String,
    pub(crate) raw_json_text: String,
    pub(crate) decoded_mail: mail_decoder::Mail,
}

pub(crate) fn process_local_file(
    input_path: &Path,
    output: &Path,
    raw_only: bool,
    format: InputFormat,
) -> Result<OutputPaths> {
    let decoded = match format {
        InputFormat::Binary => decode_binary(input_path)?,
        InputFormat::Json => decode_json(input_path)?,
    };

    process_decoded_mail(decoded, output, raw_only)
}

fn decode_binary(path: &Path) -> Result<DecodedMail> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read input file: {}", path.display()))?;
    let id = io::friendly_identifier_from_path(path);
    let decoded_mail = mail_decoder::decode(&bytes)?.into_owned();
    let raw_json_text = serde_json::to_string(&decoded_mail)?;

    Ok(DecodedMail {
        id,
        raw_json_text,
        decoded_mail,
    })
}

fn decode_json(path: &Path) -> Result<DecodedMail> {
    let raw_json_text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read input file: {}", path.display()))?;
    let decoded_mail: mail_decoder::Mail = serde_json::from_str(&raw_json_text)
        .with_context(|| format!("failed to parse mail JSON: {}", path.display()))?;
    let id = io::friendly_identifier_from_path(path);

    Ok(DecodedMail {
        id,
        raw_json_text,
        decoded_mail,
    })
}

pub(crate) fn process_decoded_mail(
    decoded: DecodedMail,
    output: &Path,
    raw_only: bool,
) -> Result<OutputPaths> {
    let decoded_mail_json: Value = serde_json::from_str(&decoded.raw_json_text)?;
    let mail_type = mail_helper::detect_mail_type(&decoded.decoded_mail);
    let include_v2 = matches!(mail_type, Some(EmailType::Battle));
    let output_paths = io::determine_output_paths(output, &decoded.id, include_v2)?;

    io::ensure_parent_dir(&output_paths.raw)?;
    io::write_json_file(&output_paths.raw, &decoded_mail_json)?;

    if raw_only {
        return Ok(output_paths);
    }

    let processed_mail_json = match mail_type {
        Some(EmailType::DuelBattle2) => {
            let processed_mail =
                mail_processor_duelbattle2::process_sections(&decoded.decoded_mail.sections)?;
            serde_json::to_value(processed_mail)?
        }
        _ => {
            let processed_mail = mail_processor::process(&decoded.raw_json_text)?;
            serde_json::to_value(processed_mail)?
        }
    };

    io::ensure_parent_dir(&output_paths.processed)?;
    io::write_json_file(&output_paths.processed, &processed_mail_json)?;

    if let Some(processed_v2_out) = &output_paths.processed_v2 {
        // Battle-only v2 processor output.
        let processed_mail =
            mail_processor_battle::process_sections(&decoded.decoded_mail.sections)?;
        let processed_mail_v2_json = serde_json::to_value(processed_mail)?;

        io::ensure_parent_dir(processed_v2_out)?;
        io::write_json_file(processed_v2_out, &processed_mail_v2_json)?;
    }

    Ok(output_paths)
}
