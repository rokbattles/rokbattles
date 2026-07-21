#![forbid(unsafe_code)]

//! Parses AllianceAOOBattleInfo mail reports.

mod body;
mod metadata;
mod rewards;

pub use rokbattles_codegen_mail_types::allianceaoobattleinfo::AllianceAooBattleInfo;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the AllianceAOOBattleInfo parser.
pub fn process(input: &Value) -> Result<AllianceAooBattleInfo, ProcessError> {
    processor().process(input)?.into_typed()
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rewards::RewardsExtractor::new()),
        Box::new(body::BodyExtractor::new()),
    ])
}
