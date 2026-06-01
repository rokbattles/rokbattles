//! Minimal protobuf descriptor decoder used by the TCP processor artifact.

mod bytes;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use self::bytes::{
    compact_bitset_value, protobuf_text_value, text_or_json_value, zlib_text_or_json_value,
};
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
        for raw in raw_fields {
            let Some(field) = fields_by_number.get(&raw.number) else {
                continue;
            };
            if let Some(value) = self.decode_field(message, field, raw.value, depth, api_map) {
                append_value(&mut out, &field.name, value);
            }
        }

        Value::Object(out)
    }

    fn decode_field(
        &self,
        _message: &Message,
        field: &Field,
        raw: RawValue,
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Option<Value> {
        let field_type = field.r#type.unwrap_or(0);
        if field_type == TYPE_MESSAGE
            && let RawValue::LengthDelimited(ref bytes) = raw
            && depth > 0
            && let Some(type_name) = field.type_name.as_deref()
            && let Some(message) = self.messages.get(normalize_name(type_name))
        {
            return Some(self.decode_message(bytes, message, depth - 1, api_map));
        }

        if field_type == TYPE_BYTES
            && let RawValue::LengthDelimited(ref bytes) = raw
            && let Some(value) = self.decode_bytes(bytes, depth, api_map)
        {
            return Some(value);
        }

        self.decode_scalar(field, raw)
    }

    fn decode_bytes(&self, bytes: &[u8], depth: usize, api_map: Option<&ApiMap>) -> Option<Value> {
        if let Some(value) = text_or_json_value(bytes) {
            return Some(value);
        }

        if let Some(value) = zlib_text_or_json_value(bytes) {
            return Some(value);
        }

        if depth > 0
            && let Some(wrapper) = parse_msg_wrapper(bytes)
        {
            let mut out = Map::new();
            out.insert("Api".to_string(), json!(wrapper.api_id));

            if let Some(mapping) = api_map.and_then(|api_map| api_map.get(wrapper.api_id)) {
                out.insert("Schema".to_string(), Value::String(mapping.schema().to_string()));
                let decoded = self.decode(mapping.descriptor(), &wrapper.payload, api_map);
                out.insert("Data".to_string(), decoded);
            } else {
                let value = protobuf_text_value(&wrapper.payload)?;
                out.insert("Data".to_string(), value);
            }

            return Some(Value::Object(out));
        }

        if depth > 0 {
            if api_map.is_some()
                && let Some(value) = self.infer_api_message_value(bytes, depth - 1, api_map)
            {
                return Some(value);
            }

            if api_map.is_none()
                && let Some(value) = self.infer_any_message_value(bytes, depth - 1, api_map)
            {
                return Some(value);
            }
        }

        if depth > 0
            && let Some(value) = protobuf_text_value(bytes)
        {
            return Some(value);
        }

        compact_bitset_value(bytes)
    }

    fn infer_api_message_value(
        &self,
        bytes: &[u8],
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Option<Value> {
        let api_map = api_map?;
        let raw_fields = parse_fields(bytes)?;
        if raw_fields.len() < 3 {
            return None;
        }
        self.infer_message_value(
            bytes,
            &raw_fields,
            depth,
            api_map.descriptor_names().filter_map(|name| self.messages.get(normalize_name(name))),
            Some(api_map),
        )
    }

    fn infer_any_message_value(
        &self,
        bytes: &[u8],
        depth: usize,
        api_map: Option<&ApiMap>,
    ) -> Option<Value> {
        let raw_fields = parse_fields(bytes)?;
        if raw_fields.len() < 3 {
            return None;
        }
        self.infer_message_value(bytes, &raw_fields, depth, self.messages.values(), api_map)
    }

    fn infer_message_value<'a>(
        &self,
        bytes: &[u8],
        raw_fields: &[crate::proto::RawField],
        depth: usize,
        candidates: impl Iterator<Item = &'a Message>,
        api_map: Option<&ApiMap>,
    ) -> Option<Value> {
        let mut best: Option<(&Message, usize)> = None;
        let mut ambiguous = false;
        for message in candidates {
            let Some(matched) = self.message_match_score(message, raw_fields, depth) else {
                continue;
            };

            let candidate = (message, matched);
            match best {
                None => {
                    best = Some(candidate);
                    ambiguous = false;
                }
                Some((_best_message, best_matched)) if matched > best_matched => {
                    best = Some(candidate);
                    ambiguous = false;
                }
                Some((best_message, best_matched))
                    if matched == best_matched
                        && message.fields.len() < best_message.fields.len() =>
                {
                    best = Some(candidate);
                    ambiguous = false;
                }
                Some((best_message, best_matched))
                    if matched == best_matched
                        && message.fields.len() == best_message.fields.len()
                        && message.full_name != best_message.full_name =>
                {
                    ambiguous = true;
                }
                Some(_) => {}
            }
        }

        if ambiguous {
            return None;
        }

        let (message, _) = best?;
        Some(self.decode_message(bytes, message, depth, api_map))
    }

    fn message_match_score(
        &self,
        message: &Message,
        raw_fields: &[crate::proto::RawField],
        depth: usize,
    ) -> Option<usize> {
        let fields_by_number: HashMap<u32, &Field> = message
            .fields
            .iter()
            .filter_map(|field| field.number.map(|number| (number, field)))
            .collect();
        if fields_by_number.is_empty() {
            return None;
        }

        let mut score = 0usize;
        for raw in raw_fields {
            let field = fields_by_number.get(&raw.number)?;
            score = score.saturating_add(self.field_match_score(&raw.value, field, depth)?);
        }
        Some(score.saturating_add(raw_fields.len()))
    }

    fn field_match_score(&self, raw: &RawValue, field: &Field, depth: usize) -> Option<usize> {
        match (field.r#type.unwrap_or(0), raw) {
            (TYPE_DOUBLE | TYPE_FIXED64 | TYPE_SFIXED64, RawValue::Fixed64(_)) => Some(8),
            (TYPE_FLOAT | TYPE_FIXED32 | TYPE_SFIXED32, RawValue::Fixed32(_)) => Some(8),
            (TYPE_BOOL, RawValue::Varint(value)) if *value <= 1 => Some(10),
            (TYPE_INT32 | TYPE_UINT32 | TYPE_ENUM | TYPE_SINT32, RawValue::Varint(value))
                if u32::try_from(*value).is_ok() =>
            {
                Some(9)
            }
            (TYPE_INT64 | TYPE_UINT64 | TYPE_SINT64, RawValue::Varint(_)) => Some(7),
            (TYPE_STRING, RawValue::LengthDelimited(bytes)) => {
                text_or_json_value(bytes).map(|_| 12)
            }
            (TYPE_MESSAGE, RawValue::LengthDelimited(bytes)) if depth > 0 => {
                let raw_fields = parse_fields(bytes)?;
                let type_name = field.type_name.as_deref()?;
                let message = self.messages.get(normalize_name(type_name))?;
                self.message_match_score(message, &raw_fields, depth - 1)
                    .map(|score| score.saturating_add(20))
            }
            (TYPE_BYTES, RawValue::LengthDelimited(_bytes)) => Some(1),
            _ => None,
        }
    }

    fn decode_scalar(&self, field: &Field, raw: RawValue) -> Option<Value> {
        let field_type = field.r#type.unwrap_or(0);
        let value = match (field_type, raw) {
            (TYPE_BOOL, RawValue::Varint(value)) => Value::Bool(value != 0),
            (TYPE_INT32, RawValue::Varint(value)) => json!((value as u32) as i32),
            (TYPE_INT64, RawValue::Varint(value)) => json!(value as i64),
            (TYPE_UINT32, RawValue::Varint(value)) => json!(value as u32),
            (TYPE_UINT64, RawValue::Varint(value)) => json!(value),
            (TYPE_ENUM, RawValue::Varint(value)) => json!(value as i32),
            (TYPE_SINT32 | TYPE_SINT64, RawValue::Varint(value)) => json!(zigzag(value)),
            (TYPE_STRING, RawValue::LengthDelimited(bytes)) => {
                let value = String::from_utf8_lossy(&bytes).into_owned();
                text_or_json_value(value.as_bytes()).unwrap_or(Value::String(value))
            }
            (TYPE_BYTES, RawValue::LengthDelimited(_bytes)) => return None,
            (TYPE_FLOAT, RawValue::Fixed32(bytes)) => json!(f32::from_le_bytes(bytes)),
            (TYPE_DOUBLE, RawValue::Fixed64(bytes)) => json!(f64::from_le_bytes(bytes)),
            (TYPE_FIXED32, RawValue::Fixed32(bytes)) => json!(u32::from_le_bytes(bytes)),
            (TYPE_SFIXED32, RawValue::Fixed32(bytes)) => json!(i32::from_le_bytes(bytes)),
            (TYPE_FIXED64, RawValue::Fixed64(bytes)) => json!(u64::from_le_bytes(bytes)),
            (TYPE_SFIXED64, RawValue::Fixed64(bytes)) => json!(i64::from_le_bytes(bytes)),
            (_, RawValue::Varint(value)) => json!(value),
            (_, RawValue::LengthDelimited(_bytes)) => return None,
            (_, RawValue::Fixed32(_bytes)) => return None,
            (_, RawValue::Fixed64(_bytes)) => return None,
        };
        Some(value)
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DescriptorArtifact {
    pub messages: Vec<Message>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_map::{ApiMap, ApiMapping};

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
    fn decode_message_parses_unmapped_json_bytes() {
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
                "Data": { "Result": [{ "Tick": 975 }] }
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_omits_opaque_bytes() {
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

        assert_eq!(value, json!({}));
    }

    #[test]
    fn decode_message_infers_embedded_protobuf_bytes_by_shape() {
        let mut messages = HashMap::new();
        messages.insert(
            "BytesContainer".to_string(),
            Message {
                name: "BytesContainer".to_string(),
                full_name: "BytesContainer".to_string(),
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
            "EmbeddedRecord".to_string(),
            Message {
                name: "EmbeddedRecord".to_string(),
                full_name: "EmbeddedRecord".to_string(),
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

        let value = descriptors.decode("BytesContainer", &payload, None);

        assert_eq!(
            value,
            json!({
                "Body": {
                    "Type": 340,
                    "Param": 1,
                    "Kvs": { "ItemId": 121 },
                }
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_omits_embedded_shape_when_ambiguous() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "BytesContainer".to_string(),
                    full_name: "BytesContainer".to_string(),
                    fields: vec![Field {
                        name: "Body".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    }],
                    nested: Vec::new(),
                },
                Message {
                    name: "FirstMatch".to_string(),
                    full_name: "FirstMatch".to_string(),
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
                            name: "Data".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
                Message {
                    name: "SecondMatch".to_string(),
                    full_name: "SecondMatch".to_string(),
                    fields: vec![
                        Field {
                            name: "Code".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Kind".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Text".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
            ],
        });
        let body = [
            0x08, 0xd4, 0x02, 0x10, 0x01, 0x1a, 0x0e, b'{', b'"', b'I', b't', b'e', b'm', b'I',
            b'd', b'"', b':', b'1', b'2', b'1', b'}',
        ];
        let mut payload = vec![0x0a, body.len() as u8];
        payload.extend_from_slice(&body);

        let value = descriptors.decode("BytesContainer", &payload, None);

        assert_eq!(value, json!({}));
    }

    #[test]
    fn decode_message_uses_api_registry_to_resolve_embedded_shape() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "BytesContainer".to_string(),
                    full_name: "BytesContainer".to_string(),
                    fields: vec![Field {
                        name: "Body".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    }],
                    nested: Vec::new(),
                },
                Message {
                    name: "RegisteredPayload".to_string(),
                    full_name: "RegisteredPayload".to_string(),
                    fields: vec![
                        Field {
                            name: "Kind".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Count".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Label".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
                Message {
                    name: "UnregisteredPayload".to_string(),
                    full_name: "UnregisteredPayload".to_string(),
                    fields: vec![
                        Field {
                            name: "Type".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Amount".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "Text".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
            ],
        });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::from([(
            "99".to_string(),
            ApiMapping {
                schema: "RegisteredPayload".to_string(),
                descriptor: "RegisteredPayload".to_string(),
            },
        )]))
        .expect("api fixture should parse");
        let body = [0x08, 0x02, 0x10, 0x03, 0x1a, 0x05, b'a', b'l', b'p', b'h', b'a'];
        let mut payload = vec![0x0a, body.len() as u8];
        payload.extend_from_slice(&body);

        let value = descriptors.decode("BytesContainer", &payload, Some(&api_map));

        assert_eq!(
            value,
            json!({
                "Body": {
                    "Kind": 2,
                    "Count": 3,
                    "Label": "alpha",
                },
            })
        );
    }

    #[test]
    fn decode_message_parses_nested_json_bytes_after_protobuf_attempt() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "OuterRecord".to_string(),
                    full_name: "OuterRecord".to_string(),
                    fields: vec![Field {
                        name: "record".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_MESSAGE),
                        type_name: Some(".BytesContainer".to_string()),
                    }],
                    nested: Vec::new(),
                },
                Message {
                    name: "BytesContainer".to_string(),
                    full_name: "BytesContainer".to_string(),
                    fields: vec![Field {
                        name: "Body".to_string(),
                        number: Some(6),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    }],
                    nested: Vec::new(),
                },
                Message {
                    name: "EmbeddedRecord".to_string(),
                    full_name: "EmbeddedRecord".to_string(),
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
            ],
        });
        let body = br#"{"Title":{"Key":"LC_EVENT_SERVER_GACHA_TITLE","List":[]}}"#;
        let mut record = vec![0x32, body.len() as u8];
        record.extend_from_slice(body);
        let mut payload = vec![0x0a, record.len() as u8];
        payload.extend_from_slice(&record);

        let value = descriptors.decode("OuterRecord", &payload, None);

        assert_eq!(
            value,
            json!({
                    "record": {
                    "Body": {
                        "Title": {
                            "Key": "LC_EVENT_SERVER_GACHA_TITLE",
                            "List": [],
                        },
                    },
                },
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_omits_unmapped_embedded_protobuf_bytes() {
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

        assert_eq!(value, json!({}));
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_unwraps_single_field_text_bytes() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "TextBodyRecord".to_string(),
                full_name: "TextBodyRecord".to_string(),
                fields: vec![Field {
                    name: "Body".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });
        let body = b"hello from wrapped text";
        let mut wrapped = vec![0x0a, body.len() as u8];
        wrapped.extend_from_slice(body);
        let mut payload = vec![0x0a, wrapped.len() as u8];
        payload.extend_from_slice(&wrapped);

        let value = descriptors.decode("TextBodyRecord", &payload, None);

        assert_eq!(value, json!({ "Body": "hello from wrapped text" }));
    }

    #[test]
    fn decode_message_parses_named_json_bytes() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "JsonBytesRecord".to_string(),
                full_name: "JsonBytesRecord".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });

        let value = descriptors.decode("JsonBytesRecord", &[0x0a, 0x02, b'{', b'}'], None);

        assert_eq!(value, json!({ "Data": {} }));
    }

    #[test]
    fn decode_message_omits_named_opaque_bytes() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "JsonBytesRecord".to_string(),
                full_name: "JsonBytesRecord".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });

        let value = descriptors.decode("JsonBytesRecord", &[0x0a, 0x03, 0xff, 0x00, 0x10], None);

        assert_eq!(value, json!({}));
    }

    #[test]
    fn decode_message_parses_key_value_text_bytes() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "KeyValueRecord".to_string(),
                full_name: "KeyValueRecord".to_string(),
                fields: vec![
                    Field {
                        name: "Code".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_ENUM),
                        type_name: Some(".ErrorCode".to_string()),
                    },
                    Field {
                        name: "Id".to_string(),
                        number: Some(2),
                        r#type: Some(TYPE_INT64),
                        type_name: None,
                    },
                    Field {
                        name: "Key".to_string(),
                        number: Some(3),
                        r#type: Some(TYPE_STRING),
                        type_name: None,
                    },
                    Field {
                        name: "Value".to_string(),
                        number: Some(4),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    },
                ],
                nested: Vec::new(),
            }],
        });
        let payload = [
            0x08, 0x01, 0x10, 0x2a, 0x1a, 0x0b, b'S', b'e', b't', b't', b'i', b'n', b'g', b's',
            b'K', b'e', b'y', 0x22, 0x07, b'7', b':', b'1', b',', b'1', b':', b'1',
        ];

        let value = descriptors.decode("KeyValueRecord", &payload, None);

        assert_eq!(
            value,
            json!({
                "Code": 1,
                "Id": 42,
                "Key": "SettingsKey",
                "Value": "7:1,1:1",
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_inflates_key_value_zlib_json_bytes() {
        use std::io::Write as _;

        use flate2::{Compression, write::ZlibEncoder};

        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "KeyValueRecord".to_string(),
                full_name: "KeyValueRecord".to_string(),
                fields: vec![
                    Field {
                        name: "Code".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_ENUM),
                        type_name: Some(".ErrorCode".to_string()),
                    },
                    Field {
                        name: "Id".to_string(),
                        number: Some(2),
                        r#type: Some(TYPE_INT64),
                        type_name: None,
                    },
                    Field {
                        name: "Key".to_string(),
                        number: Some(3),
                        r#type: Some(TYPE_STRING),
                        type_name: None,
                    },
                    Field {
                        name: "Value".to_string(),
                        number: Some(4),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    },
                ],
                nested: Vec::new(),
            }],
        });

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(br#"{"Layouts":[]}"#).expect("test fixture should compress");
        let compressed = encoder.finish().expect("test fixture should finish");

        let mut payload = vec![
            0x08,
            0x01,
            0x10,
            0x2a,
            0x1a,
            0x0a,
            b'L',
            b'a',
            b'y',
            b'o',
            b'u',
            b't',
            b'D',
            b'a',
            b't',
            b'a',
            0x22,
            compressed.len() as u8,
        ];
        payload.extend_from_slice(&compressed);

        let value = descriptors.decode("KeyValueRecord", &payload, None);

        assert_eq!(
            value,
            json!({
                "Code": 1,
                "Id": 42,
                "Key": "LayoutData",
                "Value": { "Layouts": [] },
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_parses_prefixed_zlib_bytes() {
        use std::io::Write as _;

        use flate2::{Compression, write::ZlibEncoder};

        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "Test".to_string(),
                full_name: "Test".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(br#"{"Layouts":[]}"#).expect("test fixture should compress");
        let compressed = encoder.finish().expect("test fixture should finish");
        let mut data = vec![0x01];
        data.extend_from_slice(b"088510");
        data.extend_from_slice(&compressed);
        let mut payload = vec![0x0a, data.len() as u8];
        payload.extend_from_slice(&data);

        let value = descriptors.decode("Test", &payload, None);

        assert_eq!(value, json!({ "Data": { "Layouts": [] } }));
    }

    #[test]
    fn decode_message_infers_embedded_bytes_message_from_descriptor_shape() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "ListItem".to_string(),
                    full_name: "ListItem".to_string(),
                    fields: vec![
                        Field {
                            name: "Id".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                        Field {
                            name: "Rank".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                        Field {
                            name: "Score".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                        Field {
                            name: "Value".to_string(),
                            number: Some(4),
                            r#type: Some(TYPE_BYTES),
                            type_name: None,
                        },
                        Field {
                            name: "PreviousPosition".to_string(),
                            number: Some(5),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
                Message {
                    name: "EmbeddedDetails".to_string(),
                    full_name: "EmbeddedDetails".to_string(),
                    fields: vec![
                        Field {
                            name: "Name".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                        Field {
                            name: "Tag".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                        Field {
                            name: "Image".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                        Field {
                            name: "MemberId".to_string(),
                            number: Some(4),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                        Field {
                            name: "MemberName".to_string(),
                            number: Some(5),
                            r#type: Some(TYPE_STRING),
                            type_name: None,
                        },
                        Field {
                            name: "ShardId".to_string(),
                            number: Some(11),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "AreaCount".to_string(),
                            number: Some(26),
                            r#type: Some(TYPE_INT64),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
            ],
        });
        let value_bytes = [
            0x0a, 0x05, b'G', b'r', b'o', b'u', b'p', 0x12, 0x03, b'T', b'A', b'G', 0x1a, 0x04,
            b'i', b'm', b'a', b'g', 0x20, 0x2a, 0x2a, 0x05, b'A', b'l', b'i', b'c', b'e', 0x58,
            0x0c, 0xd0, 0x01, 0x07,
        ];
        let mut payload = vec![0x08, 0x01, 0x10, 0x01, 0x18, 0x64, 0x22, value_bytes.len() as u8];
        payload.extend_from_slice(&value_bytes);
        payload.extend_from_slice(&[0x28, 0x01]);

        let value = descriptors.decode("ListItem", &payload, None);

        assert_eq!(
            value,
            json!({
                "Id": 1,
                "Rank": 1,
                "Score": 100,
                "Value": {
                    "Name": "Group",
                    "Tag": "TAG",
                    "Image": "imag",
                    "MemberId": 42,
                    "MemberName": "Alice",
                    "ShardId": 12,
                    "AreaCount": 7,
                },
                "PreviousPosition": 1,
            })
        );
    }

    #[test]
    fn decode_message_parses_compact_bitmap_bytes() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "BitmapRecord".to_string(),
                full_name: "BitmapRecord".to_string(),
                fields: vec![
                    Field {
                        name: "FogTileSize".to_string(),
                        number: Some(1),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                    Field {
                        name: "Bitmap".to_string(),
                        number: Some(2),
                        r#type: Some(TYPE_BYTES),
                        type_name: None,
                    },
                    Field {
                        name: "ServerId".to_string(),
                        number: Some(3),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                    Field {
                        name: "Width".to_string(),
                        number: Some(4),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                    Field {
                        name: "Height".to_string(),
                        number: Some(5),
                        r#type: Some(TYPE_INT32),
                        type_name: None,
                    },
                ],
                nested: Vec::new(),
            }],
        });
        let unlocked = vec![0xff; 128];
        let mut payload = vec![0x08, 0x12, 0x12, 0x80, 0x01];
        payload.extend_from_slice(&unlocked);
        payload.extend_from_slice(&[0x18, 0x8c, 0x0e, 0x20, 0xa0, 0x38, 0x28, 0xa0, 0x38]);

        let value = descriptors.decode("BitmapRecord", &payload, None);

        assert_eq!(
            value,
            json!({
                "FogTileSize": 18,
                "Bitmap": [[0, 1023]],
                "ServerId": 1804,
                "Width": 7200,
                "Height": 7200,
            })
        );
    }

    #[test]
    fn decode_message_parses_flexible_bytes_as_json() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "FlexibleBytesRecord".to_string(),
                full_name: "FlexibleBytesRecord".to_string(),
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

        let value = descriptors.decode("FlexibleBytesRecord", &payload, None);

        assert_eq!(value, json!({ "Session": "abc", "Data": { "Floor": 123 } }));
    }

    #[test]
    fn decode_message_omits_flexible_bytes_without_descriptor_match() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "FlexibleBytesRecord".to_string(),
                full_name: "FlexibleBytesRecord".to_string(),
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

        let value = descriptors.decode("FlexibleBytesRecord", &payload, None);

        assert_eq!(value, json!({}));
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
    }

    #[test]
    fn decode_message_parses_wrapped_bytes_as_inner_message() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "WrappedRecord".to_string(),
                    full_name: "WrappedRecord".to_string(),
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
                    name: "InnerRecord".to_string(),
                    full_name: "InnerRecord".to_string(),
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
            ApiMapping { schema: "InnerRecord".to_string(), descriptor: "InnerRecord".to_string() },
        )]))
        .expect("api map should load");
        let inner_payload = [0x0a, 0x03, b'k', b'v', b'k'];
        let mut wrapped_data = vec![0x08, 0xe4, 0x19, 0x12, inner_payload.len() as u8];
        wrapped_data.extend_from_slice(&inner_payload);
        let mut payload = vec![0x08, 0xcb, 0x7c, 0x12, wrapped_data.len() as u8];
        payload.extend_from_slice(&wrapped_data);

        let value = descriptors.decode("WrappedRecord", &payload, Some(&api_map));

        assert_eq!(
            value,
            json!({
                "ServerId": 15947,
                "Data": {
                    "Api": 3300,
                    "Schema": "InnerRecord",
                    "Data": { "Name": "kvk" }
                }
            })
        );
    }

    #[test]
    fn decode_message_decodes_inline_json_bytes_without_byte_summary() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![Message {
                name: "JsonBytesRecord".to_string(),
                full_name: "JsonBytesRecord".to_string(),
                fields: vec![Field {
                    name: "Data".to_string(),
                    number: Some(1),
                    r#type: Some(TYPE_BYTES),
                    type_name: None,
                }],
                nested: Vec::new(),
            }],
        });

        let value = descriptors.decode("JsonBytesRecord", &[0x0a, 0x02, b'{', b'}'], None);

        assert_eq!(value, json!({ "Data": {} }));
    }

    #[test]
    fn decode_message_decodes_inline_wrapped_bytes_without_byte_summary() {
        let descriptors = DescriptorSet::from_artifact(DescriptorArtifact {
            messages: vec![
                Message {
                    name: "WrappedRecord".to_string(),
                    full_name: "WrappedRecord".to_string(),
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
                    name: "LoginRecord".to_string(),
                    full_name: "LoginRecord".to_string(),
                    fields: vec![
                        Field {
                            name: "Schema".to_string(),
                            number: Some(1),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "ServerId".to_string(),
                            number: Some(2),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                        Field {
                            name: "TerritoryGridRadius".to_string(),
                            number: Some(3),
                            r#type: Some(TYPE_INT32),
                            type_name: None,
                        },
                    ],
                    nested: Vec::new(),
                },
            ],
        });
        let api_map = ApiMap::from_artifact(std::collections::BTreeMap::from([(
            "187".to_string(),
            ApiMapping { schema: "LoginRecord".to_string(), descriptor: "LoginRecord".to_string() },
        )]))
        .expect("api map should load");
        let payload = [
            0x08, 0xcb, 0x7c, 0x12, 0x0d, 0x08, 0xbb, 0x01, 0x12, 0x08, 0x08, 0xb3, 0x06, 0x10,
            0xcb, 0x7c, 0x18, 0x0b,
        ];

        let value = descriptors.decode("WrappedRecord", &payload, Some(&api_map));

        assert_eq!(
            value,
            json!({
                "ServerId": 15947,
                "Data": {
                    "Api": 187,
                    "Schema": "LoginRecord",
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
