#![forbid(unsafe_code)]

//! Command-line entrypoint for repository code generation.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Generates repository source artifacts.
#[derive(Debug, Parser)]
#[command(name = "rokbattles-codegen", version)]
struct Cli {
    /// TypeScript package output directory.
    #[arg(long, value_name = "DIR")]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = cli.output.unwrap_or_else(rokbattles_codegen::default_output_dir);
    rokbattles_codegen::generate(&output)
}
