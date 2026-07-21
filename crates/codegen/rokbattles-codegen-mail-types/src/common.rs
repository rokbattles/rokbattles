//! Types reused by multiple mail outputs.

use serde::{Deserialize, Serialize};
use serde_json::Number;
use ts_rs::TS;

/// Metadata present on every processed mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct MailMetadata {
    pub mail_id: String,
    pub mail_time: u64,
    pub mail_receiver: String,
    pub server_id: u64,
}

/// A two-dimensional game-map position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Position {
    #[ts(type = "number")]
    pub x: Number,
    #[ts(type = "number")]
    pub y: Number,
}

/// One item or resource reward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    pub r#type: u64,
    pub sub_type: u64,
    pub value: u64,
}
