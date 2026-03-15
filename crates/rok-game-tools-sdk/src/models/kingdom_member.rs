use serde::{Deserialize, Serialize};

use crate::models::de::{de_u32_from_string_or_int, de_u64_from_string_or_int};

/// Query parameters for `GET /api/kindomMember`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KingdomMemberRequest {
    pub start: String,
    pub end: String,
    pub search: String,
    pub server_id: String,
}

impl KingdomMemberRequest {
    /// Create a member query request.
    pub fn new(
        start: impl Into<String>,
        end: impl Into<String>,
        server_id: impl Into<String>,
    ) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            search: String::new(),
            server_id: server_id.into(),
        }
    }

    /// Set the optional search value (player id or name).
    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = search.into();
        self
    }
}

/// Response envelope for `GET /api/kindomMember`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomMemberResponse {
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub code: u32,
    pub msg: String,
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub total: u32,
    pub dt: String,
    pub data: Vec<KingdomMemberItem>,
}

/// Kingdom member stats row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomMemberItem {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub max_power: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub power: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub collect: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub kill: u64,
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
    pub help: u64,
    pub dt: String,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead_t1: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead_t2: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead_t3: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead_t4: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub dead_t5: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_search_to_empty_string() {
        let req = KingdomMemberRequest::new("2026-02-17", "2026-02-17", "2804");
        assert_eq!(req.search, "");
    }

    #[test]
    fn response_deserializes_kingdom_member_payload() {
        let json = r#"{
          "code":200,
          "msg":"OK",
          "total":1,
          "dt":"2026-02-17",
          "data":[
            {"id":"71738515","name":"Grigvar","max_power":"150164651","power":"145680111","collect":"0","dead":"0","kill":"0","t1":"0","t2":"0","t3":"0","t4":"0","t5":"0","help":"249","dt":"2026/02/17","dead_t1":"0","dead_t2":"0","dead_t3":"0","dead_t4":"0","dead_t5":"0"}
          ]
        }"#;

        let parsed: KingdomMemberResponse =
            serde_json::from_str(json).expect("parse kingdom member response");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].id, "71738515");
        assert_eq!(parsed.data[0].max_power, 150_164_651);
        assert_eq!(parsed.data[0].help, 249);
    }
}
