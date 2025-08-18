//! mail-decoder-cli: minimal convenience/debugging CLI for the Rise of Kingdoms mail decoder.
//!
//! This binary exists to quickly inspect and troubleshoot payloads. It is not intended
//! for production use; in most cases you should depend on and call the library API
//! directly from your application.
//!
//! Usage:
//!   mail-decoder-cli <file>
//!
//! Behavior:
//! - Reads the entire file into memory,
//! - Decodes it using the mail-decoder library,
//! - Prints pretty JSON to stdout,
//! - Returns a non-zero exit code on error.

use anyhow::{Context, Result};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    // Accept a single positional argument: path to the binary mail payload.
    let path = env::args()
        .nth(1)
        .context("usage: mail-decoder-cli <file>")?;
    let path = Path::new(&path);

    // Read the entire file into memory. Suitable for debugging and small samples;
    // for large inputs in production, streaming or chunked processing would be preferable.
    let bytes =
        fs::read(path).with_context(|| format!("failed to read file: {}", path.display()))?;

    // Decode the buffer using the maildecoder library.
    let value = mail_decoder::decode(&bytes)
        .with_context(|| format!("failed to decode file: {}", path.display()))?;

    // Pretty-print the resulting JSON for human inspection.
    println!("{}", serde_json::to_string_pretty(&value)?);

    Ok(())
}
