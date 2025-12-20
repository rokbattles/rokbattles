use serde::{Deserialize, Serialize};

/// Processed Battle mail output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleMail {
    pub metadata: BattleMetadata,
}

/// Metadata extracted from the raw mail sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleMetadata {
    // id
    pub email_id: Option<String>,
    // time
    pub email_time: Option<i64>,
    // receiver
    pub email_receiver: Option<String>,
    // serverId
    pub server_id: Option<i64>,
}
