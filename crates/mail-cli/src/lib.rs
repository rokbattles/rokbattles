#![forbid(unsafe_code)]

//! Support code for the `mail-cli` binary.
//!
//! The CLI reads mail buffers from an input directory, decodes them with
//! `mail-decoder`, and writes the resulting JSON next to the source files or
//! into a separate output directory.

mod config;
mod error;
mod fs_utils;
mod lossless;
mod run;

pub use config::{
    Config, MigrateConfig, MigrateSummary, RebuildConfig, RebuildSummary, RunSummary,
};
pub use error::MailCliError;
pub use lossless::{migrate_lossless, rebuild_lossless};
pub use run::run;
