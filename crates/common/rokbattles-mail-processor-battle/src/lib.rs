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

pub use rokbattles_codegen_mail_types::battle::Battle;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the Battle parser.
pub fn process(input: &Value) -> Result<Battle, ProcessError> {
    processor().process(input)?.into_typed()
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
