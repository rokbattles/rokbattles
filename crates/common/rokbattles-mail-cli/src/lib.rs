#![forbid(unsafe_code)]

//! Support code for the `rokbattles-mail-cli` binary.
//!
//! The CLI reads mail buffers from an input directory, decodes them with
//! `rokbattles-mail-decoder`, and writes the resulting JSON next to the source files or
//! into a separate output directory.

mod config;
mod error;
mod fs_utils;
mod run;

pub use config::{Config, RunSummary};
pub use error::MailCliError;
pub use run::run;
