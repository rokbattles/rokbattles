use serde::{Deserialize, Serialize};

use crate::models::de::{de_u32_from_string_or_int, de_u64_from_string_or_int};

/// Response envelope for `GET /api/kindomInformation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomInformationResponse {
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub code: u32,
    pub msg: String,
    pub data: KingdomInformationData,
}

/// Payload for kingdom information and daily changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomInformationData {
    pub name: String,
    pub day: String,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub collect: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub kill: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub power: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub t1: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub t2: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub t3: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub t4: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub t5: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_collect: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_dead: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_kill: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_power: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_t1: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_t2: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_t3: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_t4: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub change_t5: u64,
    pub dt: String,
    #[serde(rename = "kvkCnt", deserialize_with = "de_u64_from_string_or_int")]
    pub kvk_cnt: u64,
    #[serde(rename = "kvkKillScore", deserialize_with = "de_u64_from_string_or_int")]
    pub kvk_kill_score: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_deserializes_kingdom_information_payload() {
        let json = r#"{
          "code":200,
          "msg":"OK",
          "data":{
            "name":"2804",
            "day":"2026-02-17",
            "collect":"8184967869993",
            "dead":"3461833529",
            "kill":"612047619560",
            "power":"20911511398",
            "t1":"2766515157",
            "t2":"589432587",
            "t3":"655144575",
            "t4":"13306495365",
            "t5":"23731495985",
            "change_collect":"22871867579",
            "change_dead":"5014012",
            "change_kill":"170313336",
            "change_power":"86379413",
            "change_t1":"55434747",
            "change_t2":"1483436",
            "change_t3":"342946",
            "change_t4":"6365827",
            "change_t5":"4561473",
            "dt":"2026/02/17",
            "kvkCnt":"1",
            "kvkKillScore":"157900000000"
          }
        }"#;

        let parsed: KingdomInformationResponse =
            serde_json::from_str(json).expect("parse kingdom information response");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.msg, "OK");
        assert_eq!(parsed.data.name, "2804");
        assert_eq!(parsed.data.collect, 8_184_967_869_993);
        assert_eq!(parsed.data.kvk_cnt, 1);
        assert_eq!(parsed.data.kvk_kill_score, 157_900_000_000);
    }
}
