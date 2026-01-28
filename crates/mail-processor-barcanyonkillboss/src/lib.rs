#![forbid(unsafe_code)]

//! Processor for BarCanyonKillBoss mail reports.

mod content;
mod metadata;
mod npc;
mod participants;

use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

pub use mail_processor_sdk::{ExtractError, Section};

/// Process a decoded BarCanyonKillBoss mail with parallel extractors.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Process a decoded BarCanyonKillBoss mail in extractor order.
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
