//! AllianceAOORegistration output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::MailMetadata;

/// Processed AllianceAOORegistration mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct AllianceAooRegistration {
    pub metadata: MailMetadata,
    pub overview: RegistrationOverview,
    pub participants: Vec<RegistrationParticipant>,
}

/// Registration window details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RegistrationOverview {
    pub start_time: u64,
}

/// One registered alliance member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RegistrationParticipant {
    pub player_id: u64,
    pub player_name: String,
    pub power: u64,
    pub role: u64,
}
