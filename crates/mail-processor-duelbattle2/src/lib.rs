#![forbid(unsafe_code)]

//! Processor for DuelBattle2 mail reports.

mod commander;
mod metadata;
mod opponent;
mod player;
mod sender;

use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

pub use mail_processor_sdk::{ExtractError, Section};

/// Process a decoded DuelBattle2 mail with parallel extractors.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Process a decoded DuelBattle2 mail in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(opponent::OpponentExtractor::new()),
    ])
}
