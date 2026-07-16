#![forbid(unsafe_code)]

//! Command-line entrypoint for decoding mail buffers into JSON.

use std::{error::Error, path::PathBuf};

use clap::{ArgAction, Parser};
use mail_cli::{Config, MailCliError};

#[derive(Parser, Debug)]
#[command(name = "mail-cli", version, about = "Decode mail buffers into JSON")]
struct Cli {
    /// Directory that contains the mail binary files.
    #[arg(value_name = "INPUT_DIR")]
    input_dir: PathBuf,

    /// Directory where JSON output files should be written. Defaults to INPUT_DIR.
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Whether to pretty-print the JSON output.
    #[arg(long, default_value_t = true, action = ArgAction::Set, value_name = "BOOL")]
    pretty: bool,
}

fn main() {
    let Cli { input_dir, output_dir, pretty } = Cli::parse();
    let config =
        Config { output_dir: output_dir.unwrap_or_else(|| input_dir.clone()), input_dir, pretty };

    if let Err(error) = mail_cli::run(&config) {
        report_error(&error);
        std::process::exit(1);
    }
}

fn report_error(error: &MailCliError) {
    eprintln!("{error}");
    let mut source: Option<&(dyn Error + 'static)> = error.source();
    while let Some(err) = source {
        eprintln!("caused by: {err}");
        source = err.source();
    }
}
