use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedMail {
    pub metadata: Metadata,
    #[serde(rename = "self")]
    pub self_side: Participant,
    pub enemy: Participant,
    pub battle_results: BattleResults,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub email_id: Option<String>,
    pub email_time: Option<i64>,
    // empty string - unknown (might be the mail brief)
    // Alliance - alliance related mails
    // AllianceApply - people applying to join the alliance
    // AllianceBuilding - alliance building related mails
    // ArenaRankRewardReport - unknown (might be AOO reward)
    // Battle - battle reports
    // CarriageSentReport - unknown (might be resource assistance)
    // Event - event related mails
    // EventAsRank - event related mails
    // EventMemberLootReport - event related mails
    // EventPlyRank - event related mails
    // Gm - unknown
    // KillBigDreamReport - unknown
    // Mlang - unknown
    // Player - player mail sent/received mails
    // Rss - resource collected
    // ScoutReport - scout reports
    // System - system related mails
    // TeamGachaResult - unknown
    // Temple - system related mails (kingdom buffs, etc.)
    pub email_type: Option<String>,
    pub email_receiver: Option<String>,
    pub email_box: Option<String>,
    // HK/LK - gsmp, gs (gs is for older reports)
    // AOO/OL - dungeon
    pub email_role: Option<String>,

    pub is_kvk: Option<i32>,
    pub attack_id: Option<String>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,

    pub players: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commander {
    // hid/hid2
    pub id: Option<i32>,
    // hlv/hlv2
    pub level: Option<i32>,
    pub skills: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    // PId
    pub player_id: Option<i64>,
    // PName
    pub player_name: Option<String>,
    // Abbr
    pub alliance_tag: Option<String>,

    // Avatar
    pub avatar_url: Option<String>,

    // CastlePos X
    pub castle_x: Option<f64>,
    // CastlePos Y
    pub castle_y: Option<f64>,
    // IsRally
    pub is_rally: Option<i32>,

    // npc lvl
    // barb - 1-40 (for HK) - need to verify range in LK
    // barb fort - 100-110 (for HK) - need to verify range in LK
    pub npc_type: Option<i32>,
    // barb - 1
    // barb fort - 2
    pub npc_btype: Option<i32>,

    pub primary_commander: Option<Commander>,
    pub secondary_commander: Option<Commander>,

    // COSId
    pub kingdom_id: Option<i32>,

    // HEq
    pub equipment: Option<String>,
    // HFMs
    pub formation: Option<i32>,
    // HWBs (Buffs)
    pub armament_buffs: Option<String>,
    // HWBs (Affix)
    pub inscriptions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BattleResults {
    pub power: Option<i64>,
    // InitMax
    pub init_max: Option<i64>,
    // Max
    pub max: Option<i64>,
    // Healing
    pub healing: Option<i64>,
    // Death
    pub death: Option<i64>,
    // BadHurt
    pub severely_wounded: Option<i64>,
    // Hurt
    pub wounded: Option<i64>,
    // Cnt
    pub remaining: Option<i64>,
    // Gt
    pub watchtower: Option<i64>,
    // GtMax
    pub watchtower_max: Option<i64>,
    // KillScore
    pub kill_score: Option<i64>,

    pub enemy_power: Option<i64>,
    // InitMax
    pub enemy_init_max: Option<i64>,
    // Max
    pub enemy_max: Option<i64>,
    // Healing
    pub enemy_healing: Option<i64>,
    // Death
    pub enemy_death: Option<i64>,
    // BadHurt
    pub enemy_severely_wounded: Option<i64>,
    // Hurt
    pub enemy_wounded: Option<i64>,
    // Cnt
    pub enemy_remaining: Option<i64>,
    // Gt
    pub enemy_watchtower: Option<i64>,
    // GtMax
    pub enemy_watchtower_max: Option<i64>,
    // KillScore
    pub enemy_kill_score: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DecodedMail {
    pub sections: Vec<Value>,
}
