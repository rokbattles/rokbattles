use anyhow::{Context, Result};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .context("usage: mail-processor-cli <file>")?;
    let path = Path::new(&path);

    let bytes = fs::read_to_string(path)
        .with_context(|| format!("failed to read file: {}", path.display()))?;

    let value = mail_processor::process(&bytes)
        .with_context(|| format!("failed to process mail: {}", path.display()))?;

    println!("{}", serde_json::to_string_pretty(&value)?);

    Ok(())
}
