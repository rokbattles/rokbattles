use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedMail {
    pub metadata: Metadata,
    #[serde(rename = "self")]
    pub self_side: Participant,
    pub enemy: Participant,
    pub overview: OverviewResults,
    pub battle_results: BattleResults,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OverviewResults {
    // SOv -> self
    // OOv -> enemy

    // KillScore
    pub kill_score: Option<i64>,
    // BadHurt
    pub severely_wounded: Option<i64>,
    // Max
    pub max: Option<i64>,
    // Hurt
    pub wounded: Option<i64>,
    // Cnt
    pub remaining: Option<i64>,
    // Dead
    pub death: Option<i64>,

    // KillScore
    pub enemy_kill_score: Option<i64>,
    // BadHurt
    pub enemy_severely_wounded: Option<i64>,
    // Max
    pub enemy_max: Option<i64>,
    // Hurt
    pub enemy_wounded: Option<i64>,
    // Cnt
    pub enemy_remaining: Option<i64>,
    // Dead
    pub enemy_death: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    // mail id
    pub email_id: Option<String>,
    // time (epoch)
    pub email_time: Option<i64>,
    // serverId
    pub server_id: Option<i64>,
    // email_type:
    //   empty string - unknown (might be the mail brief)
    //   Alliance - alliance related mails
    //   AllianceApply - people applying to join the alliance
    //   AllianceBuilding - alliance building related mails
    //   ArenaRankRewardReport - unknown (might be AOO reward)
    //   Battle - battle reports
    //   CarriageSentReport - unknown (might be resource assistance)
    //   Event - event related mails
    //   EventAsRank - event related mails
    //   EventMemberLootReport - event related mails
    //   EventPlyRank - event related mails
    //   Gm - unknown
    //   KillBigDreamReport - unknown
    //   Mlang - unknown
    //   Player - player mail sent/received mails
    //   Rss - resource collected
    //   ScoutReport - scout reports
    //   System - system related mails
    //   TeamGachaResult - unknown
    //   Temple - system related mails (kingdom buffs, etc.)
    //   DuelBattle2 - Olympian Arena (troy kvk)
    pub email_type: Option<String>,
    pub email_receiver: Option<String>,
    // email_role:
    //   HK/LK - gsmp, gs (gs is for older reports)
    //   AOO/OL - dungeon
    pub email_role: Option<String>,

    // 1 if kvk or conquers season, otherwise 0
    pub is_kvk: Option<i32>,
    pub attack_id: Option<String>,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commander {
    // HId/HId2
    pub id: Option<i32>,
    // Hlv/HLv2
    pub level: Option<i32>,
    // HSS/HSS2
    pub skills: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NpcReward {
    pub r#type: Option<i32>,
    pub sub_type: Option<i32>,
    pub value: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    // PId
    pub player_id: Option<i64>,
    // AppUid
    pub app_uid: Option<String>,
    // CTK
    pub tracking_key: Option<String>,
    // PName
    pub player_name: Option<String>,
    // Abbr
    pub alliance_tag: Option<String>,

    // Avatar
    pub avatar_url: Option<String>,
    pub frame_url: Option<String>,

    // CastlePos X
    pub castle_x: Option<f64>,
    // CastlePos Y
    pub castle_y: Option<f64>,
    // IsRally
    pub is_rally: Option<i32>,

    // npc type
    // barb - 1-40 [need to verify]
    // barb fort - 100-110 [need to verify]
    // kvk barbs: 401 (lvl 41), 406 (lvl 46), 412 (lvl 52); likely 401..415 => lvl 41..55
    // kvk barb forts: 121 (lvl 11), 122 (lvl 12); likely 121..125 => lvl 11..15
    // Melfast (karuak ceremony): 35001 (lvl 1 hell) .. 35100 (lvl 30 hell); earlier difficulties not mapped
    pub npc_type: Option<i32>,
    // npc btype (subtype)
    // 1 -> barbs
    // 2 -> barb forts
    // 9 -> Melfast (Karuak Ceremony)
    pub npc_btype: Option<i32>,
    pub npc_rewards: Option<Vec<NpcReward>>,

    pub primary_commander: Option<Commander>,
    pub secondary_commander: Option<Commander>,

    // AbT
    // 1 -> flag
    // 3 -> stronghold
    // 11 -> horse fort (troy kvk)
    pub alliance_building: Option<i32>,

    // HEq
    pub equipment: Option<String>,
    // HEq2
    pub equipment_2: Option<String>,
    // HFMs
    pub formation: Option<i32>,
    // HWBs (Buffs)
    pub armament_buffs: Option<String>,
    // HWBs (Affix)
    pub inscriptions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BattleResults {
    // Power
    pub power: Option<i64>,
    // Contribute
    pub acclaim: Option<i64>,
    // AddCnt
    pub reinforcements_join: Option<i64>,
    // RetreatCnt
    pub reinforcements_retreat: Option<i64>,
    // SkillPower
    pub skill_power: Option<i64>,
    // AtkPower
    pub attack_power: Option<i64>,
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

    // Power
    pub enemy_power: Option<i64>,
    // Contribute
    pub enemy_acclaim: Option<i64>,
    // AddCnt
    pub enemy_reinforcements_join: Option<i64>,
    // RetreatCnt
    pub enemy_reinforcements_retreat: Option<i64>,
    // SkillPower
    pub enemy_skill_power: Option<i64>,
    // AtkPower
    pub enemy_attack_power: Option<i64>,
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
