//! ScoutReport output types.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::MailMetadata;

/// Processed ScoutReport mail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScoutReport {
    pub metadata: MailMetadata,
}
