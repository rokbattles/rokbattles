use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use serde_json::Value;
use std::{
    fs,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Decode ROK mail and output raw and processed JSON files"
)]
struct Cli {
    input: PathBuf,

    #[arg(short = 'o', long = "output")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    fs::create_dir_all(&cli.output).with_context(|| {
        format!(
            "failed to create output directory: {}",
            cli.output.display()
        )
    })?;

    let bytes = fs::read(&cli.input)
        .with_context(|| format!("failed to read input file: {}", cli.input.display()))?;

    let id = extract_mail_id(&cli.input)
        .with_context(|| format!("failed to extract id from: {}", cli.input.display()))?;

    let decoded_mail = mail_decoder::decode(&bytes)?;
    let decoded_mail_json = serde_json::to_value(decoded_mail)?;
    let decoded_mail_json_text = serde_json::to_string(&decoded_mail_json)?;

    let processed_mail = mail_processor::process(&decoded_mail_json_text)?;
    let processed_mail_json = serde_json::to_value(processed_mail)?;

    let raw_out = cli.output.join(format!("{}.json", id));
    let processed_out = cli.output.join(format!("{}-processed.json", id));

    write_json_file(&raw_out, &decoded_mail_json)
        .with_context(|| format!("failed to write raw output file: {}", raw_out.display()))?;
    write_json_file(&processed_out, &processed_mail_json).with_context(|| {
        format!(
            "failed to write processed output file: {}",
            processed_out.display()
        )
    })?;

    println!(
        "Successfully wrote files to: '{}' and '{}'",
        raw_out.display(),
        processed_out.display()
    );

    Ok(())
}

fn extract_mail_id(path: &Path) -> Result<String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("missing file name"))?
        .to_string_lossy();

    let mut parts = file_name.split('.').collect::<Vec<_>>();
    if parts.is_empty() {
        bail!("unexpected filename format");
    }
    let last = parts.pop().unwrap();

    if last.chars().all(|c| c.is_ascii_digit()) {
        return Ok(last.to_string());
    }

    if let Some(segment) = parts
        .iter()
        .rev()
        .find(|segment| segment.chars().all(|c| c.is_ascii_digit()))
    {
        return Ok((*segment).to_string());
    }

    bail!("could not find id segment in filename: {}", file_name);
}

fn write_json_file(path: &Path, value: &Value) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)?;
    Ok(())
}
