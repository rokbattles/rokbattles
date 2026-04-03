#![forbid(unsafe_code)]

//! Parses DuelBattle2 mail reports.

mod battle_results;
mod commander;
mod metadata;
mod opponent;
mod player;
mod sender;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the DuelBattle2 parser with extractors in parallel.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Runs the DuelBattle2 parser in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(opponent::OpponentExtractor::new()),
        Box::new(battle_results::BattleResultsExtractor::new()),
    ])
}
