//! SystemBarbarianFort output types.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

use crate::{MailMetadata, Position, Reward};

/// Processed SystemBarbarianFort mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SystemBarbarianFort {
    pub metadata: MailMetadata,
    pub rewards: Vec<Reward>,
    pub body: BarbarianFortBody,
}

/// Normalized barbarian fort message body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BarbarianFortBody {
    pub sub_type: u64,
    pub sub_param: u64,
    pub target_name: String,
    pub pos: Position,
    pub content: BarbarianFortContent,
}

/// Values interpolated into the localized fort message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BarbarianFortContent {
    #[ts(type = "number")]
    pub percentage: Number,
    pub tier: u64,
    pub level: u64,
}
