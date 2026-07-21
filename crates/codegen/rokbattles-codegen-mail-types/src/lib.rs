#![forbid(unsafe_code)]

//! Concrete output types for every supported mail processor.

pub mod allianceaoobattleinfo;
pub mod allianceaoobattleresults;
pub mod allianceaooindividualresults;
pub mod allianceaooregistration;
pub mod barcanyonkillboss;
pub mod battle;
mod common;
pub mod duelbattle2;
pub mod eventmemberlootreport;
pub mod rss;
pub mod scoutreport;
pub mod systembarbarianfort;
pub mod systemkahartreasure;

pub use common::{MailMetadata, Position, Reward};
use serde::{Deserialize, Serialize};

/// Any concrete processed mail output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mail {
    /// Battle report output.
    Battle(Box<battle::Battle>),
    /// Olympian Arena duel output.
    DuelBattle2(Box<duelbattle2::DuelBattle2>),
    /// Baulur report output.
    BarCanyonKillBoss(Box<barcanyonkillboss::BarCanyonKillBoss>),
    /// GVE alliance boss loot output.
    EventMemberLootReport(Box<eventmemberlootreport::EventMemberLootReport>),
    /// Resource gathering report output.
    Rss(Box<rss::Rss>),
    /// Barbarian fort reward output.
    SystemBarbarianFort(Box<systembarbarianfort::SystemBarbarianFort>),
    /// Kahar treasure reward output.
    SystemKaharTreasure(Box<systemkahartreasure::SystemKaharTreasure>),
    /// Ark of Osiris alliance result output.
    AllianceAooBattleResults(Box<allianceaoobattleresults::AllianceAooBattleResults>),
    /// Ark of Osiris schedule output.
    AllianceAooBattleInfo(Box<allianceaoobattleinfo::AllianceAooBattleInfo>),
    /// Ark of Osiris individual result output.
    AllianceAooIndividualResults(Box<allianceaooindividualresults::AllianceAooIndividualResults>),
    /// Ark of Osiris registration output.
    AllianceAooRegistration(Box<allianceaooregistration::AllianceAooRegistration>),
    /// Scout report output.
    ScoutReport(Box<scoutreport::ScoutReport>),
}
