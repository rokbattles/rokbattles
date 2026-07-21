use std::{fs, path::Path};

use serde_json::Value;

use super::{paths::output_path, process::write_processed_json};
use crate::{MailCliError, fs_utils::write_json_value};

pub(super) fn decode_file(
    input: &Path,
    output_dir: &Path,
    pretty: bool,
) -> Result<(), MailCliError> {
    let buffer =
        fs::read(input).map_err(|source| MailCliError::Io { source, path: input.to_path_buf() })?;
    let value = rokbattles_mail_decoder::decode(&buffer)
        .map_err(|source| MailCliError::Decode { source, path: input.to_path_buf() })?;
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
    write_json_value(&output_path, value, pretty)
}
