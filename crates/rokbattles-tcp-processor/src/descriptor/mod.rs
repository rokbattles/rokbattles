//! Minimal protobuf descriptor decoder used by the TCP processor artifact.

mod bytes;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use self::bytes::{
    compact_bitset_value, generic_protobuf_value, text_or_json_value, zlib_text_or_json_value,
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
        let mut unknown = Vec::new();
        for raw in raw_fields {
            let Some(field) = fields_by_number.get(&raw.number) else {
                if let Some(value) = unknown_value(raw.number, raw.wire, raw.value, depth) {
                    unknown.push(value);
                }
                continue;
            };
            if let Some(value) = self.decode_field(message, field, raw.value, depth, api_map) {
                append_value(&mut out, &field.name, value);
            }
        }

        if !unknown.is_empty() {
            out.insert("_unknown".to_string(), Value::Array(unknown));
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
                let value = generic_protobuf_value(&wrapper.payload, depth - 1)?;
                out.insert("Data".to_string(), value);
            }

            return Some(Value::Object(out));
        }

        if depth > 0
            && let Some(value) = generic_protobuf_value(bytes, depth - 1)
        {
            return Some(value);
        }

        compact_bitset_value(bytes)
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

fn unknown_value(number: u32, wire: u8, raw: RawValue, depth: usize) -> Option<Value> {
    let value = match raw {
        RawValue::Varint(value) => json!(value),
        RawValue::LengthDelimited(bytes) => {
            if let Some(value) = text_or_json_value(&bytes) {
                value
            } else if let Some(value) = zlib_text_or_json_value(&bytes) {
                value
            } else if depth > 0 {
                generic_protobuf_value(&bytes, depth - 1)?
            } else {
                return None;
            }
        }
        RawValue::Fixed32(_bytes) => return None,
        RawValue::Fixed64(_bytes) => return None,
    };
    Some(json!({ "field": number, "wire": wire, "value": value }))
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
    fn decode_message_parses_embedded_protobuf_bytes_generically() {
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
                "Body": [
                    { "Field": 1, "Wire": 0, "Value": 340 },
                    { "Field": 2, "Wire": 0, "Value": 1 },
                    { "Field": 3, "Wire": 2, "Value": { "ItemId": 121 } },
                ]
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
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
    fn decode_message_parses_unmapped_embedded_protobuf_bytes() {
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
                "Data": [
                    { "Field": 1, "Wire": 0, "Value": 340 },
                    { "Field": 2, "Wire": 0, "Value": 1 },
                    { "Field": 3, "Wire": 2, "Value": { "ItemId": 121 } },
                ]
            })
        );
        assert!(!contains_key(&value, "head_hex"));
        assert!(!contains_key(&value, "len"));
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
    fn decode_message_parses_flexible_bytes_as_generic_protobuf() {
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
