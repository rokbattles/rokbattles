#![forbid(unsafe_code)]

//! Parses AllianceAOOBattleInfo mail reports.

mod body;
mod metadata;
mod rewards;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the AllianceAOOBattleInfo parser with extractors in parallel.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Runs the AllianceAOOBattleInfo parser in extractor order.
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
