pub mod cli;
mod dir;
mod io;
mod mongo;
mod processing;

use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};

pub use cli::{Cli, Mode};

/// Entry point used by the CLI binary.
pub async fn run(cli: Cli) -> Result<()> {
    if cli.concurrency == 0 {
        bail!("concurrency must be >= 1");
    }
    match cli.mode {
        Mode::Mongo => run_mongo(cli).await,
        Mode::Binary | Mode::Json => run_local(cli).await,
    }
}

async fn run_local(cli: Cli) -> Result<()> {
    let input = match cli.input.as_ref() {
        Some(input) => PathBuf::from(input),
        None => {
            let msg = match cli.mode {
                Mode::Binary => "binary mode requires --input <path>",
                Mode::Json => "json mode requires --input <path>",
                Mode::Mongo => "mongo mode requires --input <hash>",
            };
            return Err(anyhow!(msg));
        }
    };
    let metadata = std::fs::metadata(&input)
        .with_context(|| format!("failed to read input metadata: {}", input.display()))?;

    if metadata.is_dir() {
        let output_dir = resolve_output_base(cli.output, &input, true)?;
        if output_dir.extension().is_some() {
            bail!("output must be a directory when input is a directory");
        }

        let summary = dir::process_directory(dir::DirectoryJob {
            input_dir: input,
            output_dir,
            raw_only: cli.raw_only,
            concurrency: cli.concurrency,
        })
        .await?;

        println!(
            "Processed {} mail(s) (skipped {}, failed {})",
            summary.processed, summary.skipped, summary.failed
        );
        return Ok(());
    }

    let output_base = resolve_output_base(cli.output, &input, false)?;
    let format = match cli.mode {
        Mode::Binary => processing::InputFormat::Binary,
        Mode::Json => processing::InputFormat::Json,
        Mode::Mongo => unreachable!("mongo is handled separately"),
    };

    let output_paths = processing::process_local_file(&input, &output_base, cli.raw_only, format)?;
    print_success(cli.raw_only, &output_paths);

    Ok(())
}

async fn run_mongo(cli: Cli) -> Result<()> {
    let hash = cli
        .input
        .as_ref()
        .ok_or_else(|| anyhow!("mongo mode requires --input <hash>"))?;
    let output_base = cli
        .output
        .ok_or_else(|| anyhow!("mongo mode requires --output <path>"))?;

    let (id, raw_json_text) = mongo::fetch_mail_from_mongo(hash).await?;
    let decoded_mail: mail_decoder_legacy::Mail =
        serde_json::from_str(&raw_json_text).context("failed to parse mail JSON from mongo")?;
    let decoded = processing::DecodedMail {
        id,
        raw_json_text,
        decoded_mail,
    };

    let output_paths = processing::process_decoded_mail(decoded, &output_base, cli.raw_only)?;
    print_success(cli.raw_only, &output_paths);

    Ok(())
}

fn resolve_output_base(
    output: Option<PathBuf>,
    input: &Path,
    input_is_dir: bool,
) -> Result<PathBuf> {
    match (output, input_is_dir) {
        (Some(output), _) => Ok(output),
        (None, true) => Ok(input.to_path_buf()),
        (None, false) => bail!("output path is required when input is a file"),
    }
}

fn print_success(raw_only: bool, output_paths: &io::OutputPaths) {
    if raw_only {
        println!(
            "Successfully wrote raw file to: '{}'",
            output_paths.raw.display()
        );
        return;
    }

    println!(
        "Successfully wrote files to: '{}' and '{}'",
        output_paths.raw.display(),
        output_paths.processed.display()
    );
}

#[cfg(test)]
mod tests {
    use super::{Cli, Mode};
    use crate::{dir, io, processing};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn determine_output_paths_dir() {
        let output = PathBuf::from("out");
        let paths = io::determine_output_paths(&output, "mail-1").expect("paths");

        assert_eq!(paths.raw, PathBuf::from("out/mail-1.json"));
        assert_eq!(paths.processed, PathBuf::from("out/mail-1-processed.json"));
    }

    #[test]
    fn determine_output_paths_file() {
        let output = PathBuf::from("out/mail.json");
        let paths = io::determine_output_paths(&output, "mail-3").expect("paths");

        assert_eq!(paths.raw, PathBuf::from("out/mail.json"));
        assert_eq!(paths.processed, PathBuf::from("out/mail-processed.json"));
    }

    #[test]
    fn processed_filename_detection() {
        assert!(io::is_processed_filename("foo-processed.json"));
        assert!(!io::is_processed_filename("foo.json"));
    }

    #[test]
    fn classify_path_skips_processed_outputs() {
        assert!(dir::classify_path(PathBuf::from("foo-processed.json").as_path()).is_none());
    }

    #[test]
    fn classify_path_detects_json_and_binary() {
        let json = dir::classify_path(PathBuf::from("mail.json").as_path());
        let json_caps = dir::classify_path(PathBuf::from("mail.JSON").as_path());
        let bin = dir::classify_path(PathBuf::from("mail.bin").as_path());

        assert_eq!(json, Some(processing::InputFormat::Json));
        assert_eq!(json_caps, Some(processing::InputFormat::Json));
        assert_eq!(bin, Some(processing::InputFormat::Binary));
    }

    #[test]
    fn cli_defaults_concurrency() {
        let cli = Cli::parse_from(["mail-cli", "-m", "json", "-i", "mail.json", "-o", "out"]);

        assert_eq!(cli.mode, Mode::Json);
        assert_eq!(cli.concurrency, 1);
    }

    #[test]
    fn cli_overrides_concurrency() {
        let cli = Cli::parse_from([
            "mail-cli",
            "-m",
            "json",
            "-i",
            "mail.json",
            "-o",
            "out",
            "--concurrency",
            "8",
        ]);

        assert_eq!(cli.concurrency, 8);
    }
}
