#![forbid(unsafe_code)]

//! Command-line interface for decoding mail buffers into JSON.

use std::error::Error;
use std::path::PathBuf;

use clap::{ArgAction, Parser};
use mail_cli::{Config, MailCliError, RebuildConfig};

#[derive(Parser, Debug)]
#[command(name = "mail-cli", version, about = "Decode mail buffers into JSON")]
struct Cli {
    /// Directory containing mail binary files.
    #[arg(value_name = "INPUT_DIR")]
    input_dir: PathBuf,

    /// Directory where JSON output files will be written. Defaults to INPUT_DIR.
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Whether to pretty-print JSON output.
    #[arg(long, default_value_t = true, action = ArgAction::Set, value_name = "BOOL")]
    pretty: bool,

    /// Whether to emit the lossless JSON representation.
    #[arg(long, default_value_t = false)]
    lossless: bool,

    /// Rebuild lossless JSON documents into raw mail buffers.
    #[arg(long, default_value_t = false)]
    rebuild_lossless: bool,

    /// Mail id override for rebuilding a single lossless JSON document.
    #[arg(long, value_name = "MAIL_ID")]
    mail_id: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    if cli.rebuild_lossless {
        let config = RebuildConfig {
            input_path: cli.input_dir,
            output_dir: cli.output_dir,
            mail_id: cli.mail_id,
        };
        if let Err(error) = mail_cli::rebuild_lossless(&config) {
            report_error(&error);
            std::process::exit(1);
        }
    } else {
        let output_dir = cli
            .output_dir
            .clone()
            .unwrap_or_else(|| cli.input_dir.clone());

        let config = Config {
            input_dir: cli.input_dir,
            output_dir,
            pretty: cli.pretty,
            lossless: cli.lossless,
        };

        if let Err(error) = mail_cli::run(&config) {
            report_error(&error);
            std::process::exit(1);
        }
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
