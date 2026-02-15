#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Context;
use binidx_decoder::{CatalogEntry, decode_catalog_from_encoded_tables};
use clap::{ArgGroup, Parser};

#[derive(Debug, Parser)]
#[command(
    name = "binidx-cli",
    version,
    about = "Decode and query XOR-obfuscated bin/idx tables",
    group(
        ArgGroup::new("action")
            .required(true)
            .args(["dump", "search_key", "search_value"])
    )
)]
pub struct Cli {
    /// Path to the value table (`*.bin`).
    #[arg(long, short = 'b', value_name = "PATH")]
    pub bin: PathBuf,

    /// Path to the index table (`*.idx`).
    #[arg(
        long = "index",
        visible_alias = "idx",
        short = 'i',
        value_name = "PATH"
    )]
    pub index: PathBuf,

    /// XOR key as decimal or hex (for example `50` or `0x32`).
    #[arg(long, short = 'x', value_parser = parse_u8, value_name = "KEY")]
    pub xor: u8,

    /// Dump all paired rows.
    #[arg(long)]
    pub dump: bool,

    /// Find rows where key contains query (case-insensitive).
    #[arg(long, short = 'k', value_name = "QUERY")]
    pub search_key: Option<String>,

    /// Find rows where value contains query (case-insensitive).
    #[arg(long, short = 'v', value_name = "QUERY")]
    pub search_value: Option<String>,
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let idx = std::fs::read(&cli.index)
        .with_context(|| format!("failed to read index table {}", cli.index.display()))?;
    let bin = std::fs::read(&cli.bin)
        .with_context(|| format!("failed to read value table {}", cli.bin.display()))?;

    let catalog = decode_catalog_from_encoded_tables(&idx, &bin, cli.xor).with_context(|| {
        format!(
            "failed to decode {} + {}",
            cli.index.display(),
            cli.bin.display()
        )
    })?;

    let rows = select_rows(
        &catalog.rows,
        cli.dump,
        cli.search_key.as_deref(),
        cli.search_value.as_deref(),
    );

    let mut stdout = io::stdout().lock();
    write_rows_tsv(&mut stdout, &rows)?;
    Ok(())
}

fn parse_u8(input: &str) -> Result<u8, String> {
    if let Some(hex) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        return u8::from_str_radix(hex, 16).map_err(|error| error.to_string());
    }

    input.parse::<u8>().map_err(|error| error.to_string())
}

fn select_rows<'a>(
    rows: &'a [CatalogEntry],
    dump: bool,
    search_key: Option<&str>,
    search_value: Option<&str>,
) -> Vec<&'a CatalogEntry> {
    rows.iter()
        .filter(|row| {
            if dump {
                return true;
            }

            let key_match = search_key
                .map(|query| contains_case_insensitive(&row.key, query))
                .unwrap_or(false);
            let value_match = search_value
                .map(|query| contains_case_insensitive(&row.value, query))
                .unwrap_or(false);

            key_match || value_match
        })
        .collect()
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }

    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn write_rows_tsv<W: Write>(writer: &mut W, rows: &[&CatalogEntry]) -> io::Result<()> {
    for row in rows {
        writeln!(
            writer,
            "{}\t{}\t{}",
            row.index,
            sanitize_tsv_field(&row.key),
            sanitize_tsv_field(&row.value)
        )?;
    }

    Ok(())
}

fn sanitize_tsv_field(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ if ch.is_control() => output.extend(ch.escape_default()),
            _ => output.push(ch),
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{Cli, contains_case_insensitive, select_rows, write_rows_tsv};
    use binidx_decoder::CatalogEntry;
    use clap::Parser;

    fn row(index: usize, key: &str, value: &str) -> CatalogEntry {
        CatalogEntry {
            index,
            key: key.to_owned(),
            value: value.to_owned(),
        }
    }

    fn sample_rows() -> Vec<CatalogEntry> {
        vec![
            row(1, "N_1", "Sacred Dominion"),
            row(2, "D_1", "Primary weapon"),
            row(3, "N_2", "Shield"),
        ]
    }

    #[test]
    fn cli_accepts_index_alias_and_required_flags() {
        let cli = Cli::try_parse_from([
            "binidx-cli",
            "--idx",
            "table.idx",
            "--bin",
            "table_en.bin",
            "--xor",
            "0x32",
            "--dump",
        ])
        .expect("parse args");

        assert!(cli.dump);
        assert_eq!(cli.index, PathBuf::from("table.idx"));
        assert_eq!(cli.bin, PathBuf::from("table_en.bin"));
        assert_eq!(cli.xor, 0x32);
    }

    use std::path::PathBuf;

    #[test]
    fn cli_rejects_when_no_action_is_provided() {
        let error = Cli::try_parse_from([
            "binidx-cli",
            "--index",
            "table.idx",
            "--bin",
            "table_en.bin",
            "-x",
            "50",
        ])
        .expect_err("must fail");

        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    #[test]
    fn case_insensitive_substring_matching_works() {
        assert!(contains_case_insensitive("Sacred Dominion", "dom"));
        assert!(contains_case_insensitive("Sacred Dominion", "DOM"));
        assert!(!contains_case_insensitive("Sacred Dominion", "helm"));
    }

    #[test]
    fn select_rows_supports_dump_and_search_modes() {
        let rows = sample_rows();

        let dump_rows = select_rows(&rows, true, None, None);
        assert_eq!(dump_rows.len(), 3);

        let key_rows = select_rows(&rows, false, Some("d_"), None);
        assert_eq!(key_rows.len(), 1);
        assert_eq!(key_rows[0].key, "D_1");

        let value_rows = select_rows(&rows, false, None, Some("shield"));
        assert_eq!(value_rows.len(), 1);
        assert_eq!(value_rows[0].value, "Shield");

        let either_rows = select_rows(&rows, false, Some("N_1"), Some("shield"));
        assert_eq!(either_rows.len(), 2);
        assert_eq!(either_rows[0].index, 1);
        assert_eq!(either_rows[1].index, 3);
    }

    #[test]
    fn tsv_output_sanitizes_control_characters() {
        let rows = [row(1, "N\t1", "line1\nline2")];
        let refs = rows.iter().collect::<Vec<_>>();
        let mut output = Vec::new();

        write_rows_tsv(&mut output, &refs).expect("write tsv");

        assert_eq!(
            String::from_utf8(output).expect("utf8"),
            "1\tN\\t1\tline1\\nline2\n"
        );
    }
}
