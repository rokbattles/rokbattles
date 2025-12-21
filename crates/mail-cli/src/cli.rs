use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum Mode {
    Binary,
    Json,
    Mongo,
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Decode/process ROK mail from binary/json/mongo and write raw (and optionally processed) JSON"
)]
pub struct Cli {
    /// binary (ROK mail), json (decoded JSON file), or mongo (hash lookup)
    #[arg(short = 'm', long = "mode", value_enum, default_value_t = Mode::Binary)]
    pub mode: Mode,

    #[arg(short = 'i', long = "input")]
    pub input: Option<String>,

    /// Output file or directory. Defaults to input directory when input is a directory.
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Only write the decoded/raw JSON; skip processing.
    #[arg(short = 'r', long = "raw", default_value_t = false)]
    pub raw_only: bool,

    /// Limit concurrent processing when input is a directory.
    #[arg(long = "concurrency", default_value_t = 4)]
    pub concurrency: usize,
}
