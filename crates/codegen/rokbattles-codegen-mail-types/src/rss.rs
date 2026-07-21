//! Rss output types.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

use crate::{MailMetadata, Position};

/// Processed resource-gathering mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Rss {
    pub metadata: MailMetadata,
    pub rss: ResourceGathering,
}

/// Resource-gathering details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceGathering {
    #[ts(type = "number")]
    pub rss_type: Number,
    #[ts(type = "number")]
    pub rss_value: Number,
    #[ts(type = "number")]
    pub rss_bonus: Number,
    #[ts(type = "number")]
    pub crystals_gain: Number,
    #[ts(type = "number")]
    pub level: Number,
    #[ts(type = "number")]
    pub time: Number,
    pub pos: Position,
}
