#![forbid(unsafe_code)]

//! Parses BarCanyonKillBoss mail reports.

mod content;
mod metadata;
mod npc;
mod participants;

pub use rokbattles_codegen_mail_types::barcanyonkillboss::BarCanyonKillBoss;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the BarCanyonKillBoss parser.
pub fn process(input: &Value) -> Result<BarCanyonKillBoss, ProcessError> {
    processor().process(input)?.into_typed()
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(npc::NpcExtractor::new()),
        Box::new(participants::ParticipantsExtractor::new()),
    ])
}
