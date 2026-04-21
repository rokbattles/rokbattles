use mail_decoder::{
    LosslessArray, LosslessContainer, LosslessDocument, LosslessEntry, LosslessObject,
    LosslessValue,
};
use serde_json::Value;

use super::hex::decode_hex;

pub(super) fn parse_lossless_document(value: &Value) -> Result<LosslessDocument, String> {
    let obj =
        value.as_object().ok_or_else(|| "lossless JSON root must be an object".to_string())?;
    let preamble_hex = obj
        .get("preamble_hex")
        .and_then(Value::as_str)
        .ok_or_else(|| "lossless JSON missing preamble_hex".to_string())?;
    let preamble = decode_hex(preamble_hex)?;
    let value = obj.get("value").ok_or_else(|| "lossless JSON missing value".to_string())?;
    let lossless_value = parse_lossless_value(value)?;
    Ok(LosslessDocument { preamble, value: lossless_value })
}

fn parse_lossless_value(value: &Value) -> Result<LosslessValue, String> {
    let obj = value.as_object().ok_or_else(|| "lossless value must be an object".to_string())?;
    let tag = obj
        .get("tag")
        .and_then(Value::as_str)
        .ok_or_else(|| "lossless value missing tag".to_string())?;

    match tag {
        "bool" => {
            let raw = obj
                .get("raw")
                .and_then(Value::as_u64)
                .ok_or_else(|| "lossless bool missing raw".to_string())?;
            let raw = u8::try_from(raw)
                .map_err(|_| "lossless bool raw must be in 0..=255".to_string())?;
            Ok(LosslessValue::Bool { value: raw })
        }
        "f32_le" => {
            let raw_hex = obj
                .get("raw_hex")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless f32 missing raw_hex".to_string())?;
            let bytes = decode_hex(raw_hex)?;
            if bytes.len() != 4 {
                return Err("lossless f32 raw_hex must be 4 bytes".to_string());
            }
            Ok(LosslessValue::F32 {
                raw: bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "lossless f32 raw_hex length invalid".to_string())?,
            })
        }
        "f64_be" => {
            let raw_hex = obj
                .get("raw_hex")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless f64 missing raw_hex".to_string())?;
            let bytes = decode_hex(raw_hex)?;
            if bytes.len() != 8 {
                return Err("lossless f64 raw_hex must be 8 bytes".to_string());
            }
            Ok(LosslessValue::F64 {
                raw: bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| "lossless f64 raw_hex length invalid".to_string())?,
            })
        }
        "string" => {
            let value = obj
                .get("value")
                .and_then(Value::as_str)
                .ok_or_else(|| "lossless string missing value".to_string())?;
            Ok(LosslessValue::String { value: value.to_string() })
        }
        "unknown" => {
            let raw = obj
                .get("raw")
                .and_then(Value::as_u64)
                .ok_or_else(|| "lossless unknown missing raw".to_string())?;
            let raw = u8::try_from(raw)
                .map_err(|_| "lossless unknown raw must be in 0..=255".to_string())?;
            Ok(LosslessValue::Unknown { tag: raw })
        }
        "container" => parse_container(obj),
        _ => Err(format!("lossless value has unknown tag {tag}")),
    }
}

fn parse_container(obj: &serde_json::Map<String, Value>) -> Result<LosslessValue, String> {
    let kind = obj
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "lossless container missing kind".to_string())?;
    match kind {
        "object" => parse_object_container(obj),
        "array" => parse_array_container(obj),
        _ => Err(format!("lossless container has unknown kind {kind}")),
    }
}

fn parse_object_container(obj: &serde_json::Map<String, Value>) -> Result<LosslessValue, String> {
    let entries = obj
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "lossless object missing entries".to_string())?;
    let mut parsed_entries = Vec::with_capacity(entries.len());
    for entry in entries {
        let entry_obj = entry
            .as_object()
            .ok_or_else(|| "lossless object entry must be an object".to_string())?;
        let key = entry_obj
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| "lossless object entry missing key".to_string())?;
        let value = entry_obj
            .get("value")
            .ok_or_else(|| "lossless object entry missing value".to_string())?;
        let value = parse_lossless_value(value)?;
        parsed_entries.push(LosslessEntry { key: key.to_string(), value });
    }
    let terminator = parse_terminator(obj, "object")?;
    Ok(LosslessValue::Container(LosslessContainer::Object(LosslessObject {
        entries: parsed_entries,
        terminator,
    })))
}

fn parse_array_container(obj: &serde_json::Map<String, Value>) -> Result<LosslessValue, String> {
    let items = obj
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "lossless array missing items".to_string())?;
    let mut parsed_items = Vec::with_capacity(items.len());
    for item in items {
        parsed_items.push(parse_lossless_value(item)?);
    }
    let terminator = parse_terminator(obj, "array")?;
    Ok(LosslessValue::Container(LosslessContainer::Array(LosslessArray {
        items: parsed_items,
        terminator,
    })))
}

fn parse_terminator(
    obj: &serde_json::Map<String, Value>,
    kind: &str,
) -> Result<Option<u8>, String> {
    obj.get("terminator")
        .and_then(Value::as_u64)
        .map(|raw| {
            u8::try_from(raw).map_err(|_| format!("lossless {kind} terminator must be in 0..=255"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_lossless_document_rejects_invalid_hex_length() {
        let value = json!({
            "preamble_hex": "f",
            "value": {
                "tag": "container",
                "kind": "object",
                "entries": []
            }
        });

        assert_eq!(
            parse_lossless_document(&value).unwrap_err(),
            "hex string must have even length"
        );
    }

    #[test]
    fn parse_lossless_document_rejects_invalid_hex_character() {
        let value = json!({
            "preamble_hex": "gg",
            "value": {
                "tag": "container",
                "kind": "object",
                "entries": []
            }
        });

        assert_eq!(
            parse_lossless_document(&value).unwrap_err(),
            "hex string contains invalid character g"
        );
    }

    #[test]
    fn parse_lossless_document_rejects_invalid_f32_byte_length() {
        let value = json!({
            "preamble_hex": "",
            "value": {
                "tag": "f32_le",
                "raw_hex": "010203"
            }
        });

        assert_eq!(
            parse_lossless_document(&value).unwrap_err(),
            "lossless f32 raw_hex must be 4 bytes"
        );
    }

    #[test]
    fn parse_lossless_document_rejects_invalid_f64_byte_length() {
        let value = json!({
            "preamble_hex": "",
            "value": {
                "tag": "f64_be",
                "raw_hex": "01020304050607"
            }
        });

        assert_eq!(
            parse_lossless_document(&value).unwrap_err(),
            "lossless f64 raw_hex must be 8 bytes"
        );
    }
}
