//! Minimal protobuf descriptor decoder used by the TCP processor artifact.

use std::{collections::HashMap, fmt::Write as _};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    api_map::ApiMap,
    proto::{RawValue, parse_fields, parse_msg_wrapper, zigzag},
};

const TYPE_DOUBLE: u32 = 1;
const TYPE_FLOAT: u32 = 2;
const TYPE_INT64: u32 = 3;
const TYPE_UINT64: u32 = 4;
const TYPE_INT32: u32 = 5;
const TYPE_FIXED64: u32 = 6;
const TYPE_FIXED32: u32 = 7;
const TYPE_BOOL: u32 = 8;
const TYPE_STRING: u32 = 9;
const TYPE_MESSAGE: u32 = 11;
const TYPE_BYTES: u32 = 12;
const TYPE_UINT32: u32 = 13;
const TYPE_ENUM: u32 = 14;
const TYPE_SFIXED32: u32 = 15;
const TYPE_SFIXED64: u32 = 16;
const TYPE_SINT32: u32 = 17;
const TYPE_SINT64: u32 = 18;

#[derive(Debug, Clone)]
pub struct DescriptorSet {
    messages: HashMap<String, Message>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub fields: Vec<Field>,
    #[serde(default)]
    pub nested: Vec<Message>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Field {
    pub name: String,
    pub number: Option<u32>,
    #[serde(default)]
    pub r#type: Option<u32>,
    #[serde(default)]
    pub type_name: Option<String>,
}

impl DescriptorSet {
    pub fn from_artifact(artifact: DescriptorArtifact) -> Self {
        let mut messages = HashMap::new();
        for message in artifact.messages {
            insert_message(&mut messages, message);
        }
        Self { messages }
    }

    pub fn decode(&self, message_name: &str, payload: &[u8], api_map: Option<&ApiMap>) -> Value {
        let Some(message) = self.messages.get(normalize_name(message_name)) else {
            return json!({ "error": format!("message not found: {message_name}") });
        };
        self.decode_message(payload, message, 6, api_map)
    }

    fn decode_message(
        &self,
        data: &[u8],
        message: &Message,
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Value {
        let fields_by_number: HashMap<u32, &Field> = message
            .fields
            .iter()
            .filter_map(|field| field.number.map(|number| (number, field)))
            .collect();
        let Some(raw_fields) = parse_fields(data) else {
            return json!({ "error": "payload is not valid protobuf" });
        };

        let mut out = Map::new();
        let mut unknown = Vec::new();
        for raw in raw_fields {
            let Some(field) = fields_by_number.get(&raw.number) else {
                unknown.push(unknown_value(raw.number, raw.wire, raw.value));
                continue;
            };
            let value = self.decode_field(message, field, raw.value, depth, api_map);
            append_value(&mut out, &field.name, value);
        }

        if !unknown.is_empty() {
            out.insert("_unknown".to_string(), Value::Array(unknown));
        }

        Value::Object(out)
    }

    fn decode_field(
        &self,
        message: &Message,
        field: &Field,
        raw: RawValue,
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Value {
        let field_type = field.r#type.unwrap_or(0);
        if field_type == TYPE_MESSAGE
            && let RawValue::LengthDelimited(ref bytes) = raw
            && depth > 0
            && let Some(type_name) = field.type_name.as_deref()
            && let Some(message) = self.messages.get(normalize_name(type_name))
        {
            return self.decode_message(bytes, message, depth - 1, api_map);
        }

        if field_type == TYPE_BYTES
            && let RawValue::LengthDelimited(ref bytes) = raw
            && depth > 0
            && let Some(type_name) = inferred_bytes_type_name(message, field)
            && let Some(message) = self.messages.get(type_name)
        {
            let decoded = self.decode_message(bytes, message, depth - 1, api_map);
            if !is_error_object(&decoded) {
                return decoded;
            }
        }

        if field_type == TYPE_BYTES
            && let RawValue::LengthDelimited(ref bytes) = raw
            && let Some(value) = self.decode_bytes_hint(message, field, bytes, depth, api_map)
        {
            return value;
        }

        self.decode_scalar(message, field, raw)
    }

    fn decode_bytes_hint(
        &self,
        message: &Message,
        field: &Field,
        bytes: &[u8],
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Option<Value> {
        if json_bytes_field(message, field)
            && let Some(value) = text_or_json_value(bytes)
        {
            return Some(value);
        }

        if embedded_msg_bytes_field(message, field)
            && depth > 0
            && let Some(wrapper) = parse_msg_wrapper(bytes)
        {
            let mut out = Map::new();
            out.insert("Api".to_string(), json!(wrapper.api_id));

            if let Some(mapping) = api_map.and_then(|api_map| api_map.get(wrapper.api_id)) {
                out.insert("Schema".to_string(), Value::String(mapping.schema().to_string()));
                let decoded = self.decode(mapping.descriptor(), &wrapper.payload, api_map);
                out.insert("Data".to_string(), decoded);
            } else {
                out.insert("Data".to_string(), generic_protobuf_value(&wrapper.payload, depth - 1));
            }

            return Some(Value::Object(out));
        }

        if protobuf_bytes_field(message, field) && depth > 0 {
            let value = generic_protobuf_value(bytes, depth - 1);
            if !is_bytes_summary(&value) {
                return Some(value);
            }
        }

        if resolved_bytes_field(message, field) {
            return Some(raw_bytes_fallback(bytes));
        }

        None
    }

    fn decode_scalar(&self, message: &Message, field: &Field, raw: RawValue) -> Value {
        let field_type = field.r#type.unwrap_or(0);
        match (field_type, raw) {
            (TYPE_BOOL, RawValue::Varint(value)) => Value::Bool(value != 0),
            (TYPE_INT32, RawValue::Varint(value)) => json!((value as u32) as i32),
            (TYPE_INT64, RawValue::Varint(value)) => json!(value as i64),
            (TYPE_UINT32, RawValue::Varint(value)) => json!(value as u32),
            (TYPE_UINT64, RawValue::Varint(value)) => json!(value),
            (TYPE_ENUM, RawValue::Varint(value)) => json!(value as i32),
            (TYPE_SINT32 | TYPE_SINT64, RawValue::Varint(value)) => json!(zigzag(value)),
            (TYPE_STRING, RawValue::LengthDelimited(bytes)) => {
                let value = String::from_utf8_lossy(&bytes).into_owned();
                if message.full_name == "MailSys"
                    && field.name == "Kvs"
                    && let Some(decoded) = text_or_json_value(value.as_bytes())
                {
                    return decoded;
                }
                Value::String(value)
            }
            (TYPE_BYTES, RawValue::LengthDelimited(bytes)) => bytes_value(&bytes),
            (TYPE_FLOAT, RawValue::Fixed32(bytes)) => json!(f32::from_le_bytes(bytes)),
            (TYPE_DOUBLE, RawValue::Fixed64(bytes)) => json!(f64::from_le_bytes(bytes)),
            (TYPE_FIXED32, RawValue::Fixed32(bytes)) => json!(u32::from_le_bytes(bytes)),
            (TYPE_SFIXED32, RawValue::Fixed32(bytes)) => json!(i32::from_le_bytes(bytes)),
            (TYPE_FIXED64, RawValue::Fixed64(bytes)) => json!(u64::from_le_bytes(bytes)),
            (TYPE_SFIXED64, RawValue::Fixed64(bytes)) => json!(i64::from_le_bytes(bytes)),
            (_, RawValue::Varint(value)) => json!(value),
            (_, RawValue::LengthDelimited(bytes)) => bytes_value(&bytes),
            (_, RawValue::Fixed32(bytes)) => bytes_value(&bytes),
            (_, RawValue::Fixed64(bytes)) => bytes_value(&bytes),
        }
    }
}

fn unknown_value(number: u32, wire: u8, raw: RawValue) -> Value {
    let value = match raw {
        RawValue::Varint(value) => json!(value),
        RawValue::LengthDelimited(bytes) => bytes_value(&bytes),
        RawValue::Fixed32(bytes) => bytes_value(&bytes),
        RawValue::Fixed64(bytes) => bytes_value(&bytes),
    };
    json!({ "field": number, "wire": wire, "value": value })
}

fn inferred_bytes_type_name(message: &Message, field: &Field) -> Option<&'static str> {
    match (message.full_name.as_str(), field.name.as_str()) {
        ("MailEntity", "Body") => Some("MailSys"),
        _ => None,
    }
}

fn json_bytes_field(message: &Message, field: &Field) -> bool {
    matches!(
        (message.full_name.as_str(), field.name.as_str()),
        ("ReportAck" | "ItemExtAck" | "DungeonMapNtf", "Data")
    )
}

fn embedded_msg_bytes_field(message: &Message, field: &Field) -> bool {
    matches!(
        (message.full_name.as_str(), field.name.as_str()),
        ("GuestNtf" | "DungeonMapNtf", "Data")
    )
}

fn protobuf_bytes_field(message: &Message, field: &Field) -> bool {
    embedded_msg_bytes_field(message, field)
}

fn resolved_bytes_field(message: &Message, field: &Field) -> bool {
    json_bytes_field(message, field) || protobuf_bytes_field(message, field)
}

fn is_error_object(value: &Value) -> bool {
    matches!(value, Value::Object(object) if object.contains_key("error"))
}

fn insert_message(messages: &mut HashMap<String, Message>, message: Message) {
    messages.insert(message.full_name.clone(), message.clone());
    messages.insert(message.name.clone(), message.clone());
    for nested in &message.nested {
        insert_message(messages, nested.clone());
    }
}

fn normalize_name(name: &str) -> &str {
    name.strip_prefix('.').unwrap_or(name)
}

fn text_or_json_value(bytes: &[u8]) -> Option<Value> {
    if let Ok(text) = std::str::from_utf8(bytes)
        && text.chars().all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || !ch.is_control())
    {
        let trimmed = text.trim();
        if matches!(trimmed.as_bytes().first(), Some(b'{' | b'['))
            && let Ok(value) = serde_json::from_str::<Value>(trimmed)
        {
            return Some(value);
        }
        return Some(Value::String(text.to_string()));
    }
    None
}

fn generic_protobuf_value(bytes: &[u8], depth: usize) -> Value {
    let Some(fields) = parse_fields(bytes) else {
        return bytes_value(bytes);
    };

    let values = fields
        .into_iter()
        .map(|field| {
            json!({
                "Field": field.number,
                "Wire": field.wire,
                "Value": generic_raw_value(field.value, depth),
            })
        })
        .collect();

    Value::Array(values)
}

fn generic_raw_value(raw: RawValue, depth: usize) -> Value {
    match raw {
        RawValue::Varint(value) => json!(value),
        RawValue::Fixed32(bytes) => json!(u32::from_le_bytes(bytes)),
        RawValue::Fixed64(bytes) => json!(u64::from_le_bytes(bytes)),
        RawValue::LengthDelimited(bytes) => {
            if let Some(value) = text_or_json_value(&bytes) {
                return value;
            }
            if depth > 0 {
                let value = generic_protobuf_value(&bytes, depth - 1);
                if !is_bytes_summary(&value) {
                    return value;
                }
            }
            bytes_value(&bytes)
        }
    }
}

fn is_bytes_summary(value: &Value) -> bool {
    matches!(value, Value::Object(object) if object.contains_key("head_hex"))
}

fn raw_bytes_fallback(bytes: &[u8]) -> Value {
    text_or_json_value(bytes).unwrap_or_else(|| json!({ "RawHex": bytes_to_hex(bytes) }))
}

fn append_value(out: &mut Map<String, Value>, name: &str, value: Value) {
    match out.get_mut(name) {
        Some(Value::Array(values)) => values.push(value),
        Some(existing) => {
            let first = std::mem::replace(existing, Value::Null);
            *existing = Value::Array(vec![first, value]);
        }
        None => {
            out.insert(name.to_string(), value);
        }
    }
}

fn bytes_value(bytes: &[u8]) -> Value {
    json!({ "len": bytes.len(), "head_hex": bytes_to_hex(prefix_bytes(bytes, 64)) })
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        if write!(&mut out, "{byte:02x}").is_err() {
            return out;
        }
    }
    out
}

fn prefix_bytes(bytes: &[u8], max: usize) -> &[u8] {
    bytes.get(..bytes.len().min(max)).unwrap_or(bytes)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DescriptorArtifact {
    pub messages: Vec<Message>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api_map::{ApiMap, ApiMapping},
        artifact::RuntimeArtifact,
    };

    #[test]
    fn decode_message_reads_named_fields() {
        let mut messages = HashMap::new();
        messages.insert(
            "Test".to_string(),
            Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Name".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_STRING),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };

        let value = descriptors.decode("Test", &[0x0a, 0x03, b'b', b'o', b'b'], None);

        assert_eq!(value, json!({ "Name": "bob" }));
    }

    #[test]
    fn decode_message_reads_signed_int64() {
        let mut messages = HashMap::new();
        messages.insert(
            "Test".to_string(),
            Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Score".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_INT64),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };

        let value = descriptors.decode(
            "Test",
            &[0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
            None,
        );

        assert_eq!(value, json!({ "Score": -1 }));
    }

    #[test]
    fn decode_message_summarizes_unmapped_json_bytes() {
        let mut messages = HashMap::new();
        messages.insert(
            "Test".to_string(),
            Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };
        let payload = [&[0x0a, 0x19][..], br#"{"Result":[{"Tick":975}]}"#.as_slice()].concat();

        let value = descriptors.decode("Test", &payload, None);

        assert_eq!(
            value,
            json!({
                "Data": {
                    "len": 25,
                    "head_hex": bytes_to_hex(br#"{"Result":[{"Tick":975}]}"#)
                }
            })
        );
    }

    #[test]
    fn decode_message_summarizes_large_text_bytes() {
        let text = "a".repeat(4097);
        let value = bytes_value(text.as_bytes());

        assert_eq!(
            value,
            json!({
                "len": 4097,
                "head_hex": bytes_to_hex(prefix_bytes(text.as_bytes(), 64))
            })
        );
    }

    #[test]
    fn decode_message_summarizes_opaque_bytes() {
        let mut messages = HashMap::new();
        messages.insert(
            "Test".to_string(),
            Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };
        let payload = [0x0a, 0x03, 0xff, 0x00, 0x10];

        let value = descriptors.decode("Test", &payload, None);

        assert_eq!(value, json!({ "Data": { "len": 3, "head_hex": "ff0010" } }));
    }

    #[test]
    fn decode_message_infers_mail_entity_body_as_mail_sys() {
        let mut messages = HashMap::new();
        messages.insert(
            "MailEntity".to_string(),
            Message {
                name: "MailEntity".to_string(),
                full_name: "MailEntity".to_string(),
                fields: vec![Field {
                    name: "Body".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        messages.insert(
            "MailSys".to_string(),
            Message {
                name: "MailSys".to_string(),
                full_name: "MailSys".to_string(),
                fields: vec![
                    Field {
                        name: "Type".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                    Field {
                        name: "Param".to_string(),
                        number: Some(2),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                    Field {
                        name: "Kvs".to_string(),
                        number: Some(3),
                        r#type: Some(TYPE_STRING),
                        type_name: None,
                    },
                ],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };
        let body = [
            0x08, 0xd4, 0x02, 0x10, 0x01, 0x1a, 0x0e, b'{', b'"', b'I', b't', b'e', b'm', b'I',
            b'd', b'"', b':', b'1', b'2', b'1', b'}',
        ];
        let mut payload = vec![0x0a, body.len() as u8];
        payload.extend_from_slice(&body);

        let value = descriptors.decode("MailEntity", &payload, None);

        assert_eq!(
            value,
            json!({
                "Body": {
                    "Type": 340,
                    "Param": 1,
                    "Kvs": { "ItemId": 121 }
                }
            })
        );
    }

    #[test]
    fn decode_message_summarizes_unmapped_embedded_protobuf_bytes() {
        let mut messages = HashMap::new();
        messages.insert(
            "Test".to_string(),
            Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            },
        );
        let descriptors = DescriptorSet { messages };
        let data = [
            0x08, 0xd4, 0x02, 0x10, 0x01, 0x1a, 0x0e, b'{', b'"', b'I', b't', b'e', b'm', b'I',
            b'd', b'"', b':', b'1', b'2', b'1', b'}',
        ];
        let mut payload = vec![0x0a, data.len() as u8];
        payload.extend_from_slice(&data);

        let value = descriptors.decode("Test", &payload, None);

        assert_eq!(
            value,
            json!({
                "Data": {
                    "len": 21,
                    "head_hex": bytes_to_hex(&data)
                }
            })
        );
    }

    #[test]
    fn decode_message_parses_item_ext_ack_data_as_json() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "ItemExtAck".to_string(),
                full_name: "ItemExtAck".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });

        let value = descriptors.decode("ItemExtAck", &[0x0a, 0x02, b'{', b'}'], None);

        assert_eq!(value, json!({ "Data": {} }));
    }

    #[test]
    fn decode_message_does_not_summarize_target_bytes_when_payload_is_opaque() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "ItemExtAck".to_string(),
                full_name: "ItemExtAck".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });

        let value = descriptors.decode("ItemExtAck", &[0x0a, 0x03, 0xff, 0x00, 0x10], None);

        assert_eq!(value, json!({ "Data": { "RawHex": "ff0010" } }));
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_parses_dungeon_map_ntf_data_as_json() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "DungeonMapNtf".to_string(),
                full_name: "DungeonMapNtf".to_string(),
                fields: vec![
                    Field {
                        name: "Session".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_STRING),
                        type_name: None,
                    },
                    Field {
                        name: "Data".to_string(),
                        number: Some(3),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    },
                ],
                nested: Vec::new(),
            }],
        });
        let payload = [
            0x0a, 0x03, b'a', b'b', b'c', 0x1a, 0x0d, b'{', b'"', b'F', b'l', b'o', b'o', b'r',
            b'"', b':', b'1', b'2', b'3', b'}',
        ];

        let value = descriptors.decode("DungeonMapNtf", &payload, None);

        assert_eq!(value, json!({ "Session": "abc", "Data": { "Floor": 123 } }));
    }

    #[test]
    fn decode_message_parses_dungeon_map_ntf_data_as_generic_protobuf() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "DungeonMapNtf".to_string(),
                full_name: "DungeonMapNtf".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(3),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });
        let payload = [0x1a, 0x02, 0x08, 0x7b];

        let value = descriptors.decode("DungeonMapNtf", &payload, None);

        assert_eq!(
            value,
            json!({
                "Data": [
                    { "Field": 1, "Wire": 0, "Value": 123 }
                ]
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_parses_guest_ntf_data_as_inner_msg() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "GuestNtf".to_string(),
                    full_name: "GuestNtf".to_string(),
                    fields: vec![
                        Field {
                            name: "ServerId".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Data".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_BYTES),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
                Message {
                    name: "LostLandInfoNtf".to_string(),
                    full_name: "LostLandInfoNtf".to_string(),
                    fields: vec![Field {
                        name: "Name".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_STRING),
                        type_name: None,
                    }],
                    nested: Vec::new(),
                },
            ],
        });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::from([(
            "3300".to_string(),
            ApiMapping {
                schema: "LostLandInfoNtf".to_string(),
                descriptor: "LostLandInfoNtf".to_string(),
            },
        )]))
        .expect("api map should load");
        let inner_payload = [0x0a, 0x03, b'k', b'v', b'k'];
        let mut guest_data = vec![0x08, 0xe4, 0x19, 0x12, inner_payload.len() as u8];
        guest_data.extend_from_slice(&inner_payload);
        let mut payload = vec![0x08, 0xcb, 0x7c, 0x12, guest_data.len() as u8];
        payload.extend_from_slice(&guest_data);

        let value = descriptors.decode("GuestNtf", &payload, Some(&api_map));

        assert_eq!(
            value,
            json!({
                "ServerId": 15947,
                "Data": {
                    "Api": 3300,
                    "Schema": "LostLandInfoNtf",
                    "Data": { "Name": "kvk" }
                }
            })
        );
    }

    #[test]
    fn default_artifact_decodes_sample_item_ext_ack_without_byte_summary() {
        let artifact = RuntimeArtifact::load_default().expect("runtime artifact should load");

        let value = artifact.descriptors.decode(
            "ItemExtAck",
            &[0x0a, 0x02, b'{', b'}'],
            Some(&artifact.api_map),
        );

        assert_eq!(value, json!({ "Data": {} }));
    }

    #[test]
    fn default_artifact_decodes_sample_guest_ntf_without_byte_summary() {
        let artifact = RuntimeArtifact::load_default().expect("runtime artifact should load");
        let payload = [
            0x08, 0xcb, 0x7c, 0x12, 0x0d, 0x08, 0xbb, 0x01, 0x12, 0x08, 0x08, 0xb3, 0x06, 0x10,
            0xcb, 0x7c, 0x18, 0x0b,
        ];

        let value = artifact.descriptors.decode("GuestNtf", &payload, Some(&artifact.api_map));

        assert_eq!(
            value,
            json!({
                "ServerId": 15947,
                "Data": {
                    "Api": 187,
                    "Schema": "GuestLoginNtf",
                    "Data": {
                        "Schema": 819,
                        "ServerId": 15947,
                        "TerritoryGridRadius": 11,
                    }
                }
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    fn contains_key(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(object) => {
                object.contains_key(key) || object.values().any(|value| contains_key(value, key))
            }
            Value::Array(values) => values.iter().any(|value| contains_key(value, key)),
            _ => false,
        }
    }
}
