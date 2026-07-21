//! SystemKaharTreasure output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MailMetadata, Reward};

/// Processed SystemKaharTreasure mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SystemKaharTreasure {
    pub metadata: MailMetadata,
    pub loot: Vec<Reward>,
}
