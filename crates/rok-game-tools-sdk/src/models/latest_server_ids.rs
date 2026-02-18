use crate::models::de::de_u32_from_string_or_int;
use serde::{Deserialize, Serialize};

/// Response envelope for `GET /api/latestServerIds`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestServerIdsResponse {
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub code: u32,
    pub msg: String,
    pub data: LatestServerIdsData,
}

/// Payload for latest server ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestServerIdsData {
    pub dt: String,
    pub server_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_deserializes_latest_server_ids_payload() {
        let json = r#"{
          "code":200,
          "msg":"OK",
          "data":{
            "dt":"2026/02/17",
            "server_ids":["1001","1002","1003"]
          }
        }"#;

        let parsed: LatestServerIdsResponse =
            serde_json::from_str(json).expect("parse latest server ids response");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.msg, "OK");
        assert_eq!(parsed.data.dt, "2026/02/17");
        assert_eq!(parsed.data.server_ids.len(), 3);
        assert_eq!(parsed.data.server_ids[0], "1001");
    }
}
