use serde::{Deserialize, Serialize};

/// Processed Battle mail output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleMail {
    pub metadata: BattleMetadata,
}

/// Metadata extracted from the raw mail sections.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleMetadata {
    // __rokb_email_type (home | kvk | ark)
    // ark: Role = dungeon (priority, always ark when Role = dungeon)
    // kvk: isConquerSeason = true OR serverId != sender kingdom (COSId); also Role = gsmp or gs (gs is for older reports)
    // home: serverId == sender kingdom (COSId) & Role = gsmp or gs (gs is for older reports)
    #[serde(rename = "__rokb_email_type")]
    pub rokb_email_type: Option<String>,
    // __rokb_battle_type (open_field | rally | garrison)
    // open_field: sender doesn't contain AbT field & IsRally is false or missing
    // rally: sender contains IsRally = true
    // garrison: sender contains AbT field (not present unless its a garrison)
    #[serde(rename = "__rokb_battle_type")]
    pub rokb_battle_type: Option<String>,

    // id
    pub email_id: Option<String>,
    // time
    pub email_time: Option<i64>,
    // receiver
    pub email_receiver: Option<String>,
    // serverId
    pub server_id: Option<i64>,
}
