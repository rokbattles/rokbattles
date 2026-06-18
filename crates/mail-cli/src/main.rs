#![forbid(unsafe_code)]

//! Command-line entrypoint for mail decode and migration tools.

use std::{error::Error, path::PathBuf};

use clap::{ArgAction, Parser};
use mail_cli::{Config, MailCliError, MigrateConfig, RebuildConfig};

#[derive(Parser, Debug)]
#[command(name = "mail-cli", version, about = "Decode mail buffers into JSON")]
struct Cli {
    /// Directory or file to read.
    #[arg(value_name = "INPUT")]
    input_path: PathBuf,

    /// Directory where output files should be written. Defaults to INPUT.
    #[arg(long, value_name = "OUTPUT_DIR")]
    output_dir: Option<PathBuf>,

    /// Whether to pretty-print the JSON output.
    #[arg(long, default_value_t = true, action = ArgAction::Set, value_name = "BOOL")]
    pretty: bool,

    /// Whether to emit lossless JSON instead of the standard decoded form.
    #[arg(long, default_value_t = false, conflicts_with = "binary_cursor")]
    lossless: bool,

    /// Decode binary mail files with `binary-cursor`.
    #[arg(long = "binary-cursor", default_value_t = false)]
    binary_cursor: bool,

    /// Rebuild lossless JSON documents back into raw mail buffers.
    #[arg(long = "rebuild", default_value_t = false, conflicts_with = "migrate")]
    rebuild: bool,

    /// Convert lossless JSON documents into binary-cursor v2 JSON.
    #[arg(long = "migrate", default_value_t = false, conflicts_with = "rebuild")]
    migrate: bool,

    /// Mail ID override when rebuilding a single lossless JSON document.
    #[arg(long, value_name = "MAIL_ID")]
    mail_id: Option<String>,
}

fn main() {
    let Cli { input_path, output_dir, pretty, lossless, binary_cursor, rebuild, migrate, mail_id } =
        Cli::parse();

    if rebuild {
        let config = RebuildConfig { input_path, output_dir, mail_id };
        if let Err(error) = mail_cli::rebuild_lossless(&config) {
            report_error(&error);
            std::process::exit(1);
        }
    } else if migrate {
        let config = MigrateConfig { input_path, output_dir, pretty };
        if let Err(error) = mail_cli::migrate_lossless(&config) {
            report_error(&error);
            std::process::exit(1);
        }
    } else {
        let config = Config {
            output_dir: output_dir.unwrap_or_else(|| input_path.clone()),
            input_dir: input_path,
            pretty,
            lossless,
            binary_cursor,
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
