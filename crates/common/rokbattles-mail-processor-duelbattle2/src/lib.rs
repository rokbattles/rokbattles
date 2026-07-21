#![forbid(unsafe_code)]

//! Parses DuelBattle2 mail reports.

mod battle_results;
mod commander;
mod metadata;
mod opponent;
mod player;
mod sender;

pub use rokbattles_codegen_mail_types::duelbattle2::DuelBattle2;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the DuelBattle2 parser.
pub fn process(input: &Value) -> Result<DuelBattle2, ProcessError> {
    processor().process(input)?.into_typed()
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(opponent::OpponentExtractor::new()),
        Box::new(battle_results::BattleResultsExtractor::new()),
    ])
}
