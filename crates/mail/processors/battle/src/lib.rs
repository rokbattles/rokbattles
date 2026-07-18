#![forbid(unsafe_code)]

//! Parses Battle mail reports.

mod battle_effects;
mod content;
mod metadata;
mod opponents;
mod participants;
mod player;
mod sender;
mod summary;
mod timeline;

pub use mail_sdk::{ExtractError, Section};
use mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the Battle parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(summary::SummaryExtractor::new()),
        Box::new(opponents::OpponentsExtractor::new()),
        Box::new(timeline::TimelineExtractor::new()),
    ])
}
