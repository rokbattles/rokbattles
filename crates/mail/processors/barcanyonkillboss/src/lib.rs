#![forbid(unsafe_code)]

//! Parses BarCanyonKillBoss mail reports.

mod content;
mod metadata;
mod npc;
mod participants;

pub use mail_sdk::{ExtractError, Section};
use mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the BarCanyonKillBoss parser.
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(npc::NpcExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}
