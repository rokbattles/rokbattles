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
    pub hash: Option<String>,

    pub email_id: Option<String>,
    pub email_time: Option<i64>,
    pub email_type: Option<String>,
    pub email_receiver: Option<String>,
    pub email_box: Option<String>,
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
    pub id: Option<i32>,
    pub level: Option<i32>,
    pub skills: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Participant {
    pub player_id: Option<i64>,
    pub player_name: Option<String>,
    pub alliance_tag: Option<String>,
    pub ct: Option<i32>,

    pub castle_x: Option<f64>,
    pub castle_y: Option<f64>,

    pub is_rally: Option<i32>,

    pub npc_type: Option<i32>,
    pub npc_btype: Option<i32>,

    pub primary_commander: Option<Commander>,
    pub secondary_commander: Option<Commander>,

    pub COSId: Option<i32>,
    pub CId: Option<i64>,
    pub CtId: Option<i64>,
    pub Idt: Option<i64>,
    pub AId: Option<i64>,
    pub tour_id: Option<String>,

    pub equipment: Option<String>,
    pub formation: Option<i32>,
    pub armament_buffs: Option<String>,
    pub inscriptions: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BattleResults {
    pub power: Option<i64>,
    pub init_max: Option<i64>,
    pub max: Option<i64>,
    pub healing: Option<i64>,
    pub death: Option<i64>,
    pub severely_wounded: Option<i64>,
    pub wounded: Option<i64>,
    pub count: Option<i64>,
    pub kill_score: Option<i64>,

    pub enemy_power: Option<i64>,
    pub enemy_init_max: Option<i64>,
    pub enemy_max: Option<i64>,
    pub enemy_healing: Option<i64>,
    pub enemy_death: Option<i64>,
    pub enemy_severely_wounded: Option<i64>,
    pub enemy_wounded: Option<i64>,
    pub enemy_count: Option<i64>,
    pub enemy_kill_score: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DecodedMail {
    pub sections: Vec<Value>,
}
