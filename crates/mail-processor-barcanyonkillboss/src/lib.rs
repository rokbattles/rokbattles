#![forbid(unsafe_code)]

//! Parses BarCanyonKillBoss mail reports.

mod content;
mod metadata;
mod npc;
mod participants;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the BarCanyonKillBoss parser with extractors in parallel.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Runs the BarCanyonKillBoss parser in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(npc::NpcExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}
