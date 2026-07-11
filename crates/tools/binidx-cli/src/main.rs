#![forbid(unsafe_code)]

use binidx_cli::{Cli, run};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run(cli)
}
