#![forbid(unsafe_code)]

//! Extracts structured sections from decoded Olympian Arena (DuelBattle2) mail.
//!
//! Pass the decoded root object to [`process`]. The caller selects the mail
//! category; this crate does not decode binary files or validate the root `type`
//! label. Field names are case-sensitive.
//!
//! # Sections
//!
//! | Section | Shape | Contents |
//! | --- | --- | --- |
//! | `metadata` | object | Root `id`, `time`, `receiver`, and `serverId` under the standard SDK field names. |
//! | `sender` | object | Attacking player identity, commanders, and buffs from `body.detail.AtkPlayer`. |
//! | `opponent` | object | Defending player identity, commanders, and buffs from `body.detail.DefPlayer`. |
//! | `battle_results` | object | Per-side outcome, kill points, power loss, and troop counters. |
//!
//! Both player objects are required. Each side must provide `Heroes.MainHero`,
//! `Heroes.AssistHero`, their skill arrays, and `Heroes.Buffs`, including when an
//! array is empty. Numeric counters are unsigned integers; `IsWin` and `Awaked`
//! are booleans. Values are renamed without deriving an outcome from the counters.
//!
//! # Examples
//!
//! Process an already-decoded JSON report:
//!
//! ```no_run
//! use rokbattles_mail_processor_duelbattle2::process;
//! use serde_json::Value;
//!
//! let input: Value = serde_json::from_slice(&std::fs::read("mail.json")?)?;
//! let output = process(&input)?;
//! println!("{}", serde_json::to_string_pretty(&output)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod battle_results;
mod commander;
mod metadata;
mod opponent;
mod player;
mod sender;

pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Extracts the sections described in the [crate documentation](crate#sections).
///
/// Borrows `input` and returns owned section data. The SDK runs independent
/// section extractors on scoped threads; no partial output is returned on error.
///
/// # Errors
///
/// Returns [`ProcessError::ExtractorFailed`] with the section name and original
/// [`ExtractError`] when a required value is absent or invalid. Optional fields
/// use the format-specific defaults described above; other invalid values fail
/// extraction. Worker and section-name failures follow the SDK's
/// [`Processor::process`] behavior.
///
/// # Panics
///
/// Has the thread-spawning and panic-propagation behavior of [`Processor::process`].
pub fn process(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(opponent::OpponentExtractor::new()),
        Box::new(battle_results::BattleResultsExtractor::new()),
    ])
}
