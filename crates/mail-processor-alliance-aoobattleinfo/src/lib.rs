#![forbid(unsafe_code)]

//! Processor for AllianceAOOBattleInfo mail reports.

mod body;
mod metadata;
mod rewards;

use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

pub use mail_processor_sdk::{ExtractError, Section};

/// Process a decoded AllianceAOOBattleInfo mail with parallel extractors.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Process a decoded AllianceAOOBattleInfo mail in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor::new()),
    ])
}
