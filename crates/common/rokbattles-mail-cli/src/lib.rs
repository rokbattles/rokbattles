#![forbid(unsafe_code)]

//! Decodes a directory of persistent mail files and processes recognized mail.
//!
//! [`run`] reads each selected file with [`rokbattles_mail_decoder::decode`] and
//! writes its decoded JSON. For object roots recognized by the mail registry,
//! it also writes the category processor's output to a separate JSON file.
//! Unknown categories and non-object roots still receive decoded output.
//!
//! # Files and output
//!
//! Only files directly inside the input directory are selected. Files whose
//! extension is `json`, ignoring ASCII case, are skipped; every other file is
//! attempted, including files without an extension. Paths are sorted before
//! decoding, and subdirectories are not traversed.
//!
//! Output names retain the complete input filename:
//!
//! | Input | Decoded output | Processed output, when recognized |
//! | --- | --- | --- |
//! | `sample.mail` | `sample.mail.json` | `sample.mail-processed.json` |
//! | `Persistent.Mail.123` | `Persistent.Mail.123.json` | `Persistent.Mail.123-processed.json` |
//!
//! The output directory is created if necessary. Existing output files are
//! overwritten. Processing stops at the first error, leaving completed writes
//! in place; decoded JSON is written before category processing begins. Skipping
//! an unrecognized category does not remove an older processed output file.
//!
//! # Command-line usage
//!
//! ```text
//! rokbattles-mail-cli ./mails --output-dir ./decoded --pretty false
//! ```
//!
//! The binary defaults to the input directory for output and pretty-printed
//! JSON. `--pretty` takes an explicit boolean value. Library callers supply all
//! settings through [`Config`]. A successful CLI run is silent; failures print
//! the error chain to standard error and exit with status 1.
//!
//! # Contributor guide
//!
//! `main` parses arguments and reports errors. `run` coordinates directory
//! traversal; its `decode` and `process` modules write the two representations.
//! `run::paths` defines file selection and naming, while `fs_utils` handles
//! directory access and JSON serialization. Category-specific extraction belongs
//! to the processors selected by the registry.

mod config;
mod error;
mod fs_utils;
mod run;

pub use config::{Config, RunSummary};
pub use error::MailCliError;
pub use run::run;
