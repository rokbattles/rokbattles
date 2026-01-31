#![forbid(unsafe_code)]

//! Shared SDK for mail processors.
//!
//! The SDK provides extractor traits, processor orchestration, and typed helpers
//! for pulling values out of decoded mail JSON.

mod error;
mod extract;
mod processor;
mod types;

pub use error::{ExtractError, ProcessError};
pub use extract::{indexed_array_values, require_object, require_string, require_u64};
pub use processor::{Extractor, Processor};
pub use types::{ProcessedMail, Section};
