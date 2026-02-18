use crate::error::RokGtError;
use crate::models::{
    KingdomInformationResponse, KingdomListResponse, KingdomMemberResponse, LatestServerIdsResponse,
};
use reqwest::StatusCode;
use serde::Deserialize;
use serde::de::DeserializeOwned;

pub(super) trait ApiEnvelope {
    fn code(&self) -> u32;
    fn message(&self) -> &str;
}

impl ApiEnvelope for KingdomListResponse {
    fn code(&self) -> u32 {
        self.code
    }

    fn message(&self) -> &str {
        &self.msg
    }
}

impl ApiEnvelope for LatestServerIdsResponse {
    fn code(&self) -> u32 {
        self.code
    }

    fn message(&self) -> &str {
        &self.msg
    }
}

impl ApiEnvelope for KingdomInformationResponse {
    fn code(&self) -> u32 {
        self.code
    }

    fn message(&self) -> &str {
        &self.msg
    }
}

impl ApiEnvelope for KingdomMemberResponse {
    fn code(&self) -> u32 {
        self.code
    }

    fn message(&self) -> &str {
        &self.msg
    }
}

pub(super) fn parse_api_response<T>(status: StatusCode, bytes: &[u8]) -> Result<T, RokGtError>
where
    T: DeserializeOwned + ApiEnvelope,
{
    if !status.is_success() {
        return Err(parse_non_success_response(status, bytes));
    }

    let payload: T = serde_json::from_slice(bytes).map_err(RokGtError::Decode)?;
    if payload.code() != 200 {
        return Err(RokGtError::Api {
            code: payload.code(),
            message: payload.message().to_string(),
        });
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
struct HttpErrorEnvelope {
    message: Option<String>,
    error: Option<String>,
    #[serde(rename = "statusCode")]
    status_code: Option<u16>,
}

fn parse_non_success_response(status: StatusCode, bytes: &[u8]) -> RokGtError {
    if let Ok(payload) = serde_json::from_slice::<HttpErrorEnvelope>(bytes) {
        let message = payload
            .message
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                payload
                    .error
                    .as_deref()
                    .map(str::trim)
                    .filter(|m| !m.is_empty())
                    .map(ToOwned::to_owned)
            });
        if let Some(message) = message {
            return RokGtError::Api {
                code: payload
                    .status_code
                    .map(u32::from)
                    .unwrap_or(status.as_u16().into()),
                message,
            };
        }
    }

    RokGtError::HttpStatus {
        status,
        body: body_preview(bytes),
    }
}

fn body_preview(body: &[u8]) -> String {
    const MAX_CHARS: usize = 2048;
    let decoded = String::from_utf8_lossy(body);
    let mut chars = decoded.chars();
    let preview: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_maps_non_success_http_status() {
        let err = parse_api_response::<KingdomListResponse>(
            StatusCode::BAD_GATEWAY,
            b"upstream unavailable",
        )
        .expect_err("status should fail");
        assert!(matches!(
            err,
            RokGtError::HttpStatus {
                status: StatusCode::BAD_GATEWAY,
                ..
            }
        ));
    }

    #[test]
    fn parse_maps_api_code_errors() {
        let body = json!({
            "code": 401,
            "data": [],
            "dt": "2026/02/17",
            "msg": "token invalid",
            "page": 1,
            "pages": 1,
            "size": 12,
            "total": 0
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<KingdomListResponse>(StatusCode::OK, &encoded)
            .expect_err("api error");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 401,
                message
            } if message == "token invalid"
        ));
    }

    #[test]
    fn parse_maps_json_decode_errors() {
        let err = parse_api_response::<KingdomListResponse>(StatusCode::OK, b"{\"code\":200")
            .expect_err("decode error");
        assert!(matches!(err, RokGtError::Decode(_)));
    }

    #[test]
    fn parse_latest_server_ids_success() {
        let body = json!({
            "code": 200,
            "msg": "OK",
            "data": {
                "dt": "2026/02/17",
                "server_ids": ["1001", "1002", "1003"]
            }
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let parsed = parse_api_response::<LatestServerIdsResponse>(StatusCode::OK, &encoded)
            .expect("parse latest ids");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.msg, "OK");
        assert_eq!(parsed.data.server_ids, vec!["1001", "1002", "1003"]);
    }

    #[test]
    fn parse_latest_server_ids_maps_api_code_errors() {
        let body = json!({
            "code": 403,
            "msg": "forbidden",
            "data": {
                "dt": "2026/02/17",
                "server_ids": []
            }
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<LatestServerIdsResponse>(StatusCode::OK, &encoded)
            .expect_err("api code should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 403,
                message
            } if message == "forbidden"
        ));
    }

    #[test]
    fn parse_kingdom_information_success() {
        let body = json!({
            "code": 200,
            "msg": "OK",
            "data": {
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
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let parsed = parse_api_response::<KingdomInformationResponse>(StatusCode::OK, &encoded)
            .expect("parse kingdom information");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.msg, "OK");
        assert_eq!(parsed.data.name, "2804");
        assert_eq!(parsed.data.kvk_kill_score, 157_900_000_000);
    }

    #[test]
    fn parse_kingdom_information_maps_api_code_errors() {
        let body = json!({
            "code": 500,
            "msg": "internal error",
            "data": {
                "name":"2804",
                "day":"2026-02-17",
                "collect":"0",
                "dead":"0",
                "kill":"0",
                "power":"0",
                "t1":"0",
                "t2":"0",
                "t3":"0",
                "t4":"0",
                "t5":"0",
                "change_collect":"0",
                "change_dead":"0",
                "change_kill":"0",
                "change_power":"0",
                "change_t1":"0",
                "change_t2":"0",
                "change_t3":"0",
                "change_t4":"0",
                "change_t5":"0",
                "dt":"2026/02/17",
                "kvkCnt":"0",
                "kvkKillScore":"0"
            }
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<KingdomInformationResponse>(StatusCode::OK, &encoded)
            .expect_err("api code should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 500,
                message
            } if message == "internal error"
        ));
    }

    #[test]
    fn parse_kingdom_member_success() {
        let body = json!({
            "code":200,
            "msg":"OK",
            "total":1,
            "dt":"2026-02-17",
            "data":[
                {"id":"71738515","name":"Grigvar","max_power":"150164651","power":"145680111","collect":"0","dead":"0","kill":"0","t1":"0","t2":"0","t3":"0","t4":"0","t5":"0","help":"249","dt":"2026/02/17","dead_t1":"0","dead_t2":"0","dead_t3":"0","dead_t4":"0","dead_t5":"0"}
            ]
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let parsed = parse_api_response::<KingdomMemberResponse>(StatusCode::OK, &encoded)
            .expect("parse kingdom member");
        assert_eq!(parsed.code, 200);
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.data[0].id, "71738515");
        assert_eq!(parsed.data[0].help, 249);
    }

    #[test]
    fn parse_kingdom_member_maps_api_code_errors() {
        let body = json!({
            "code": 429,
            "msg": "too many requests",
            "total": 0,
            "dt": "2026-02-17",
            "data": []
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<KingdomMemberResponse>(StatusCode::OK, &encoded)
            .expect_err("api code should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 429,
                message
            } if message == "too many requests"
        ));
    }

    #[test]
    fn parse_kingdom_member_maps_missing_auth_http_error_payload() {
        let body = json!({
            "message": "PUP_TOKEN_MISSING",
            "error": "Unauthorized",
            "statusCode": 401
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<KingdomMemberResponse>(StatusCode::UNAUTHORIZED, &encoded)
            .expect_err("auth payload should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 401,
                message
            } if message == "PUP_TOKEN_MISSING"
        ));
    }

    #[test]
    fn parse_latest_server_ids_maps_missing_auth_http_error_payload() {
        let body = json!({
            "message": "PUP_TOKEN_MISSING",
            "error": "Unauthorized",
            "statusCode": 401
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<LatestServerIdsResponse>(StatusCode::UNAUTHORIZED, &encoded)
            .expect_err("auth payload should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 401,
                message
            } if message == "PUP_TOKEN_MISSING"
        ));
    }

    #[test]
    fn parse_non_success_falls_back_to_error_when_message_blank() {
        let body = json!({
            "message": "",
            "error": "Unauthorized",
            "statusCode": 401
        });
        let encoded = serde_json::to_vec(&body).expect("encode body");
        let err = parse_api_response::<LatestServerIdsResponse>(StatusCode::UNAUTHORIZED, &encoded)
            .expect_err("auth payload should fail");
        assert!(matches!(
            err,
            RokGtError::Api {
                code: 401,
                message
            } if message == "Unauthorized"
        ));
    }
}
