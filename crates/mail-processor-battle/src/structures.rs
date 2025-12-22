use serde::{Deserialize, Serialize};

/// Processed Battle mail output.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleMail {
    pub metadata: BattleMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_summary: Option<DataSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battle_trends: Option<BattleTrends>,
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
    // TODO (future): add swarm (maybe swarm_rally & swarm_garrison), city-related attacks like city_swarm, city_garrison, city_rally
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

/// Aggregated overview stats for sender and opponent.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DataSummary {
    // SOv KillScore
    pub sender_kill_points: Option<i64>,
    // SOv BadHurt
    pub sender_severely_wounded: Option<i64>,
    // SOv Hurt
    pub sender_slightly_wounded: Option<i64>,
    // SOv Max
    pub sender_troop_units: Option<i64>,
    // SOv Cnt
    pub sender_remaining: Option<i64>,
    // SOv Dead
    pub sender_dead: Option<i64>,

    // OOv KillScore
    pub opponent_kill_points: Option<i64>,
    // OOv BadHurt
    pub opponent_severely_wounded: Option<i64>,
    // OOv Hurt
    pub opponent_slightly_wounded: Option<i64>,
    // OOv Max
    pub opponent_troop_units: Option<i64>,
    // OOv Cnt
    pub opponent_remaining: Option<i64>,
    // OOv Dead
    pub opponent_dead: Option<i64>,
}

/// Time-series sampling and event data from the battle report.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleTrends {
    // Samples
    pub sampling: Vec<BattleSampling>,
    // Events
    pub events: Vec<BattleEvent>,
}

/// Sampling snapshot extracted from the "Samples" object.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleSampling {
    // Cnt
    pub count: Option<i64>,
    // T
    pub tick: Option<i64>,
}

/// Event snapshot extracted from the "Events" object.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleEvent {
    // Et (18 = reinforcement join, 26 = reinforcement leave).
    pub r#type: Option<i64>,
    // T
    pub tick: Option<i64>,
    // AssistUnits
    pub reinforcements: Option<BattleAssistUnits>,
}

/// Assist unit metadata carried by a battle event.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleAssistUnits {
    // PId
    pub player_id: Option<i64>,
    // PName
    pub player_name: Option<String>,
    // Avatar.avatar
    pub avatar_url: Option<String>,
    // Avatar.avatarFrame
    pub frame_url: Option<String>,
    pub commanders: Option<BattleCommanders>,
}

/// Commander metadata for assist units.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleCommanders {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<BattleCommander>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<BattleCommander>,
}

/// Commander details for assist units.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleCommander {
    // HId or HId2
    pub id: Option<i64>,
    // HLv or HLv2
    pub level: Option<i64>,
}
