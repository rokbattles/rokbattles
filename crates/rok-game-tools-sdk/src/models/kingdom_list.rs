use crate::models::de::{de_u32_from_string_or_int, de_u64_from_string_or_int};
use serde::{Deserialize, Serialize};

/// Supported sorting keys for kingdom ranking queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KingdomOrderBy {
    #[default]
    Power,
    Time,
}

/// Query parameters for `GET /api/kindomList`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KingdomListRequest {
    pub page: u32,
    pub size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    pub order_by: KingdomOrderBy,
}

impl Default for KingdomListRequest {
    fn default() -> Self {
        Self {
            page: 1,
            size: 12,
            server_id: None,
            order_by: KingdomOrderBy::Power,
        }
    }
}

/// Response envelope for the kingdom list endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomListResponse {
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub code: u32,
    pub data: Vec<KingdomListItem>,
    pub dt: String,
    pub msg: String,
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub page: u32,
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub pages: u32,
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub size: u32,
    #[serde(deserialize_with = "de_u32_from_string_or_int")]
    pub total: u32,
}

/// Grade values used by kingdom ranking dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KingdomGrade {
    #[serde(rename = "S")]
    S,
    #[serde(rename = "A")]
    A,
    #[serde(rename = "B")]
    B,
    #[serde(rename = "C")]
    C,
    #[serde(rename = "D")]
    D,
}

/// Kingdom ranking entry returned by the endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomListItem {
    pub activity: KingdomGrade,
    pub begin: String,
    pub development: KingdomGrade,
    pub field: KingdomGrade,
    pub garrison: KingdomGrade,
    pub name: String,
    pub power: KingdomGrade,
    pub process: String,
    pub rank: String,
    pub server: String,
    pub technology: KingdomGrade,
    #[serde(deserialize_with = "de_u64_from_string_or_int")]
    pub total_power: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_defaults_are_expected() {
        let req = KingdomListRequest::default();
        assert_eq!(req.page, 1);
        assert_eq!(req.size, 12);
        assert_eq!(req.server_id, None);
        assert_eq!(req.order_by, KingdomOrderBy::Power);
    }

    #[test]
    fn response_deserializes_sample_payload() {
        let json = r#"{
          "code":200,
          "data":[{"activity":"S","begin":"1539545851","development":"S","field":"S","garrison":"S","name":"93","power":"S","process":"2026-02-17","rank":"1","server":"1093","technology":"S","total_power":32908810505}],
          "dt":"2026/02/17",
          "msg":"Request response successful",
          "page":1,
          "pages":228,
          "size":12,
          "total":2726
        }"#;

        let parsed: KingdomListResponse = serde_json::from_str(json).expect("parse sample");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.page, 1);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].server, "1093");
        assert_eq!(parsed.data[0].total_power, 32908810505);
    }

    #[test]
    fn response_tolerates_string_encoded_numerics() {
        let json = r#"{
          "code":"200",
          "data":[{"activity":"A","begin":"1","development":"B","field":"A","garrison":"A","name":"1","power":"S","process":"2026-02-17","rank":"3","server":"1001","technology":"B","total_power":"28077934286"}],
          "dt":"2026/02/17",
          "msg":"ok",
          "page":"1",
          "pages":"2",
          "size":"12",
          "total":"24"
        }"#;

        let parsed: KingdomListResponse = serde_json::from_str(json).expect("parse mixed numerics");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.pages, 2);
        assert_eq!(parsed.total, 24);
        assert_eq!(parsed.data[0].total_power, 28077934286);
    }

    #[test]
    fn response_with_empty_data_is_supported() {
        let json = r#"{
          "code":200,
          "msg":"Request response successful",
          "data":[],
          "total":0,
          "page":1,
          "size":12,
          "pages":0,
          "dt":"2026/02/17"
        }"#;

        let parsed: KingdomListResponse = serde_json::from_str(json).expect("parse empty response");
        assert_eq!(parsed.code, 200);
        assert!(parsed.data.is_empty());
        assert_eq!(parsed.total, 0);
        assert_eq!(parsed.pages, 0);
    }

    #[test]
    fn response_rejects_invalid_grade_value() {
        let json = r#"{
          "code":200,
          "data":[{"activity":"X","begin":"1","development":"B","field":"A","garrison":"A","name":"1","power":"S","process":"2026-02-17","rank":"3","server":"1001","technology":"B","total_power":"28077934286"}],
          "dt":"2026/02/17",
          "msg":"ok",
          "page":"1",
          "pages":"2",
          "size":"12",
          "total":"24"
        }"#;

        let err = serde_json::from_str::<KingdomListResponse>(json).expect_err("invalid grade");
        assert!(err.to_string().contains("unknown variant"));
    }
}
