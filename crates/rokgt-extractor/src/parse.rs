use reqwest::StatusCode;
use serde_json::{Map, Value};

use crate::{date::date_to_iso_2_utc, error::RokGtError, models::KingdomMember, util::truncate};

pub(crate) async fn read_api_data(response: reqwest::Response) -> Result<Value, RokGtError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        if status == StatusCode::UNAUTHORIZED
            && let Some(message) = auth_error_message(&body)
        {
            return Err(RokGtError::AuthRequired { message });
        }
        return Err(RokGtError::HttpStatus { status, body: truncate(&body, 512) });
    }

    let value = serde_json::from_str::<Value>(&body)?;
    if let Some(code) = value.get("code").and_then(Value::as_i64)
        && code != 200
    {
        let message =
            value.get("msg").and_then(Value::as_str).unwrap_or("unknown api error").to_string();
        return Err(RokGtError::Api { code, message });
    }

    value.get("data").cloned().ok_or(RokGtError::MissingField("data"))
}

fn auth_error_message(body: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let status_code = value.get("statusCode").and_then(Value::as_u64);
    let error = value.get("error").and_then(Value::as_str);
    let message = value.get("message").and_then(Value::as_str)?;
    let is_pup_token_error = matches!(message, "PUP_TOKEN_MISSING" | "PUP_TOKEN_INVALID");
    if status_code == Some(401) && error == Some("Unauthorized") && is_pup_token_error {
        return Some(message.to_string());
    }
    None
}

pub(crate) fn parse_server_ids(data: &Value) -> Result<Vec<u32>, RokGtError> {
    data.as_object()
        .and_then(|object| object.get("server_ids"))
        .and_then(parse_server_id_array)
        .ok_or(RokGtError::InvalidServerIds)
}

pub(crate) fn parse_member_records(
    server_id: u32,
    data: &Value,
) -> Result<Vec<KingdomMember>, RokGtError> {
    let members = data.as_array().ok_or(RokGtError::InvalidMembers(server_id))?;

    let mut records = Vec::with_capacity(members.len());
    for member in members {
        let mut fields = member.as_object().ok_or(RokGtError::InvalidMembers(server_id))?.clone();
        normalize_member_date(&mut fields)?;
        records.push(KingdomMember { kingdom: server_id, fields });
    }
    Ok(records)
}

fn normalize_member_date(fields: &mut Map<String, Value>) -> Result<(), RokGtError> {
    let Some(dt) = fields.remove("dt").and_then(|value| value.as_str().map(ToString::to_string))
    else {
        return Ok(());
    };
    fields.insert("date".to_string(), Value::String(date_to_iso_2_utc(&dt)?));
    Ok(())
}

fn parse_server_id_array(value: &Value) -> Option<Vec<u32>> {
    let array = value.as_array()?;
    let mut ids = Vec::with_capacity(array.len());
    for item in array {
        let id = value_to_u32(item)?;
        ids.push(id);
    }
    Some(ids)
}

fn value_to_u32(value: &Value) -> Option<u32> {
    if let Some(number) = value.as_u64() {
        return u32::try_from(number).ok();
    }
    value.as_str()?.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_server_ids_accepts_live_server_ids_shape() {
        let data = serde_json::json!({
            "dt": "2026/05/24",
            "server_ids": ["1001", "1002", "4104"]
        });

        let ids = parse_server_ids(&data).expect("server ids should parse");

        assert_eq!(ids, vec![1001, 1002, 4104]);
    }

    #[test]
    fn parse_member_records_injects_kingdom() {
        let data = serde_json::json!([
            {"id": "1", "name": "Example", "power": 10}
        ]);

        let members = parse_member_records(2804, &data).expect("members should parse");
        let first = members.first().expect("member should exist");

        assert_eq!(first.kingdom, 2804);
        assert_eq!(first.fields.get("id").and_then(Value::as_str), Some("1"));
    }

    #[test]
    fn parse_member_records_accepts_live_kindom_member_shape() {
        let data = serde_json::json!([
            {
                "id": "1",
                "name": "Example",
                "max_power": 2000000,
                "power": 2000000,
                "collect": 2000000,
                "dead": 0,
                "kill": 0,
                "t1": 0,
                "t2": 0,
                "t3": 0,
                "t4": 0,
                "t5": 0,
                "help": 5,
                "dt": "2026/05/24",
                "dead_t1": 0,
                "dead_t2": 0,
                "dead_t3": 0,
                "dead_t4": 0,
                "dead_t5": 0
            }
        ]);

        let members = parse_member_records(1001, &data).expect("members should parse");
        let first = members.first().expect("member should exist");

        assert_eq!(first.kingdom, 1001);
        assert_eq!(first.fields.get("date").and_then(Value::as_str), Some("2026-05-24T02:00:00Z"));
        assert!(!first.fields.contains_key("dt"));
    }

    #[test]
    fn auth_error_message_accepts_live_missing_token_shape() {
        let body = r#"{"message":"PUP_TOKEN_MISSING","error":"Unauthorized","statusCode":401}"#;

        assert_eq!(auth_error_message(body).as_deref(), Some("PUP_TOKEN_MISSING"));
    }

    #[test]
    fn auth_error_message_accepts_live_invalid_token_shape() {
        let body = r#"{"message":"PUP_TOKEN_INVALID","error":"Unauthorized","statusCode":401}"#;

        assert_eq!(auth_error_message(body).as_deref(), Some("PUP_TOKEN_INVALID"));
    }
}
