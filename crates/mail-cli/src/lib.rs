#![forbid(unsafe_code)]

//! Library helpers for the `mail-cli` binary.
//!
//! The CLI scans an input directory for mail binary buffers, decodes each buffer
//! into JSON using the `mail-decoder` crate, and writes JSON files alongside the
//! input data (or to a specified output directory).

mod config;
mod error;
mod fs_utils;
mod lossless;
mod run;

pub use config::{Config, RebuildConfig, RebuildSummary, RunSummary};
pub use error::MailCliError;
pub use lossless::rebuild_lossless;
pub use run::run;
