use serde::{Deserialize, Serialize};

/// Processed DuelBattle2 mail output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Mail {
    pub metadata: DuelBattle2Metadata,
}

/// Metadata extracted from the raw mail sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DuelBattle2Metadata {
    // id
    pub email_id: Option<String>,
    // time
    pub email_time: Option<i64>,
    // type
    pub email_type: Option<String>,
    // box
    pub email_box: Option<String>,
    // sender
    pub email_sender: Option<String>,
    // receiver
    pub email_receiver: Option<String>,
    // serverId
    pub server_id: Option<i64>,
}
