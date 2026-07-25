//! Startup loader for the runtime protocol artifact.

use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::Deserialize;

const CURRENT_SCHEMA_VERSION: u32 = 1;
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
const ARTIFACT_PATH: &str = "artifacts/artifacts.json";

pub(crate) const HANDSHAKE_API_ID: u32 = 8562;
pub(crate) const COMPRESSED_API_ID: u32 = 9999;
pub(crate) const ZMSG_API_ID: u32 = 61438;

const CARRIER_APIS: [(u32, &str); 4] =
    [(7901, "MailGetAck"), (7909, "MailNtf"), (7921, "MailsNtf"), (7927, "MailCheckAck")];

/// Validated protocol fields retained from the runtime artifact.
#[derive(Debug)]
pub struct RuntimeArtifact {
    pub(crate) protocol: ProtocolSchema,
    pub(crate) carriers: HashMap<u32, CarrierSchema>,
}

impl RuntimeArtifact {
    /// Load and validate the runtime artifact from its fixed path.
    ///
    /// Only the protocol fields needed by the live observer remain in memory
    /// after validation.
    ///
    /// # Errors
    ///
    /// Returns [`ArtifactError`] when the file cannot be read, exceeds the
    /// configured bound, contains invalid JSON, or does not describe the
    /// expected protocol relationships.
    pub fn load_default() -> Result<Self, ArtifactError> {
        let runtime_path = PathBuf::from(ARTIFACT_PATH);
        if runtime_path.is_file() {
            return Self::load(&runtime_path);
        }
        Self::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join(runtime_path))
    }

    fn load(path: &Path) -> Result<Self, ArtifactError> {
        let raw = read_bounded(path)?;
        let file: RuntimeArtifactFile =
            serde_json::from_slice(&raw).map_err(ArtifactError::Json)?;
        Self::from_file(file)
    }

    /// Number of mail carrier APIs validated during startup.
    #[must_use]
    pub fn carrier_count(&self) -> usize {
        self.carriers.len()
    }

    fn from_file(file: RuntimeArtifactFile) -> Result<Self, ArtifactError> {
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ArtifactError::UnsupportedSchemaVersion {
                actual: file.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }

        let messages = MessageIndex::new(&file.descriptors.messages)?;
        let msg = messages.required("Msg")?;
        let handshake = messages.for_api(&file.api_map, HANDSHAKE_API_ID, "SxNtf")?;
        let compressed = messages.for_api(&file.api_map, COMPRESSED_API_ID, "CompressedMsg")?;
        let zmsg = messages.for_api(&file.api_map, ZMSG_API_ID, "ZMsg")?;
        let report_ack = messages.required("ReportAck")?;
        let mail_entity = messages.required("MailEntity")?;

        let protocol = ProtocolSchema {
            msg_api_field: msg.required_field("Api", FieldType::Int32)?.number,
            msg_payload_field: msg.required_field("Payload", FieldType::Bytes)?.number,
            handshake_key1_field: handshake.required_field("K1", FieldType::Int32)?.number,
            handshake_key2_field: handshake.required_field("K2", FieldType::Int32)?.number,
            compressed: CompressionSchema {
                length_field: compressed.required_field("Len", FieldType::Int32)?.number,
                payload_field: compressed.required_field("Data", FieldType::Bytes)?.number,
            },
            zmsg: CompressionSchema {
                length_field: zmsg.required_field("Len", FieldType::Int32)?.number,
                payload_field: zmsg.required_field("ZData", FieldType::Bytes)?.number,
            },
            report_data_field: report_ack.required_field("Data", FieldType::Bytes)?.number,
            mail_entity: MessageShape::from_descriptor(mail_entity)?,
        };

        let mut carriers = HashMap::with_capacity(CARRIER_APIS.len());
        for (api_id, expected_descriptor) in CARRIER_APIS {
            let message = messages.for_api(&file.api_map, api_id, expected_descriptor)?;
            let entity_fields = message
                .fields
                .iter()
                .filter(|field| {
                    field.field_type == FieldType::Message.code()
                        && normalize_type_name(&field.type_name) == "MailEntity"
                })
                .collect::<Vec<_>>();
            let [entity_field] = entity_fields.as_slice() else {
                return Err(ArtifactError::Invalid(
                    "mail carrier must contain exactly one MailEntity field",
                ));
            };
            carriers.insert(
                api_id,
                CarrierSchema {
                    entity_field: entity_field.number,
                    shape: MessageShape::from_descriptor(message)?,
                },
            );
        }

        Ok(Self { protocol, carriers })
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        let message_field = WireRule { primary: 2, packed: false };
        let mut carriers = HashMap::new();
        for (api_id, _descriptor) in CARRIER_APIS {
            carriers.insert(
                api_id,
                CarrierSchema {
                    entity_field: 1,
                    shape: MessageShape {
                        fields: HashMap::from([
                            (1, message_field),
                            (2, WireRule { primary: 0, packed: false }),
                        ]),
                    },
                },
            );
        }
        Self {
            protocol: ProtocolSchema {
                msg_api_field: 1,
                msg_payload_field: 2,
                handshake_key1_field: 1,
                handshake_key2_field: 2,
                compressed: CompressionSchema { length_field: 1, payload_field: 2 },
                zmsg: CompressionSchema { length_field: 1, payload_field: 2 },
                report_data_field: 1,
                mail_entity: MessageShape {
                    fields: HashMap::from([
                        (1, WireRule { primary: 2, packed: false }),
                        (6, WireRule { primary: 2, packed: false }),
                        (9, WireRule { primary: 2, packed: false }),
                    ]),
                },
            },
            carriers,
        }
    }
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ArtifactError> {
    let file = File::open(path)
        .map_err(|source| ArtifactError::Read { path: path.to_path_buf(), source })?;
    let mut raw = Vec::new();
    file.take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|source| ArtifactError::Read { path: path.to_path_buf(), source })?;
    if raw.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(ArtifactError::TooLarge { path: path.to_path_buf(), max: MAX_ARTIFACT_BYTES });
    }
    Ok(raw)
}

/// Startup failures while loading the runtime artifact.
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    /// The artifact could not be read.
    #[error("failed to read runtime artifact {}: {source}", path.display())]
    Read {
        /// Artifact path.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// The artifact exceeded the startup size bound.
    #[error("runtime artifact {} exceeds the {max}-byte limit", path.display())]
    TooLarge {
        /// Artifact path.
        path: PathBuf,
        /// Maximum accepted file size.
        max: u64,
    },
    /// The artifact was not valid JSON.
    #[error("runtime artifact contains invalid JSON: {0}")]
    Json(#[source] serde_json::Error),
    /// The generator and relay disagree on the artifact format.
    #[error("unsupported runtime artifact schema version {actual}; expected {expected}")]
    UnsupportedSchemaVersion {
        /// Version read from the file.
        actual: u32,
        /// Version supported by this relay.
        expected: u32,
    },
    /// A required protocol relationship was absent or incompatible.
    #[error("invalid runtime artifact: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Deserialize)]
struct RuntimeArtifactFile {
    schema_version: u32,
    api_map: BTreeMap<String, ApiMapping>,
    descriptors: DescriptorArtifact,
}

#[derive(Debug, Deserialize)]
struct ApiMapping {
    descriptor: String,
}

#[derive(Debug, Deserialize)]
struct DescriptorArtifact {
    messages: Vec<DescriptorMessage>,
}

#[derive(Debug, Deserialize)]
struct DescriptorMessage {
    name: String,
    full_name: String,
    fields: Vec<DescriptorField>,
}

impl DescriptorMessage {
    fn required_field(
        &self,
        name: &str,
        field_type: FieldType,
    ) -> Result<&DescriptorField, ArtifactError> {
        let fields = self
            .fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(name))
            .collect::<Vec<_>>();
        let [field] = fields.as_slice() else {
            return Err(ArtifactError::Invalid(
                "required descriptor field is missing or duplicated",
            ));
        };
        if field.field_type != field_type.code() {
            return Err(ArtifactError::Invalid("descriptor field has an incompatible type"));
        }
        Ok(field)
    }
}

#[derive(Debug, Deserialize)]
struct DescriptorField {
    name: String,
    number: u32,
    label: u8,
    #[serde(rename = "type")]
    field_type: u8,
    #[serde(default)]
    type_name: String,
}

#[derive(Debug)]
struct MessageIndex<'a> {
    messages: HashMap<&'a str, &'a DescriptorMessage>,
}

impl<'a> MessageIndex<'a> {
    fn new(messages: &'a [DescriptorMessage]) -> Result<Self, ArtifactError> {
        let mut index = HashMap::with_capacity(messages.len());
        for message in messages {
            for name in [&*message.name, normalize_type_name(&message.full_name)] {
                if let Some(previous) = index.insert(name, message)
                    && !std::ptr::eq(previous, message)
                {
                    return Err(ArtifactError::Invalid("descriptor message name is duplicated"));
                }
            }
        }
        Ok(Self { messages: index })
    }

    fn required(&self, name: &str) -> Result<&'a DescriptorMessage, ArtifactError> {
        self.messages
            .get(normalize_type_name(name))
            .copied()
            .ok_or(ArtifactError::Invalid("required descriptor message is missing"))
    }

    fn for_api(
        &self,
        api_map: &BTreeMap<String, ApiMapping>,
        api_id: u32,
        expected_descriptor: &str,
    ) -> Result<&'a DescriptorMessage, ArtifactError> {
        let mapping = api_map
            .get(&api_id.to_string())
            .ok_or(ArtifactError::Invalid("required API mapping is missing"))?;
        if normalize_type_name(&mapping.descriptor) != expected_descriptor {
            return Err(ArtifactError::Invalid("required API maps to an unexpected descriptor"));
        }
        self.required(&mapping.descriptor)
    }
}

fn normalize_type_name(name: &str) -> &str {
    name.trim_start_matches('.')
}

#[derive(Debug, Clone, Copy)]
enum FieldType {
    Int32,
    Message,
    Bytes,
}

impl FieldType {
    const fn code(self) -> u8 {
        match self {
            Self::Int32 => 5,
            Self::Message => 11,
            Self::Bytes => 12,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProtocolSchema {
    pub(crate) msg_api_field: u32,
    pub(crate) msg_payload_field: u32,
    pub(crate) handshake_key1_field: u32,
    pub(crate) handshake_key2_field: u32,
    pub(crate) compressed: CompressionSchema,
    pub(crate) zmsg: CompressionSchema,
    pub(crate) report_data_field: u32,
    pub(crate) mail_entity: MessageShape,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompressionSchema {
    pub(crate) length_field: u32,
    pub(crate) payload_field: u32,
}

#[derive(Debug)]
pub(crate) struct CarrierSchema {
    pub(crate) entity_field: u32,
    pub(crate) shape: MessageShape,
}

#[derive(Debug)]
pub(crate) struct MessageShape {
    fields: HashMap<u32, WireRule>,
}

impl MessageShape {
    fn from_descriptor(message: &DescriptorMessage) -> Result<Self, ArtifactError> {
        let mut fields = HashMap::with_capacity(message.fields.len());
        for field in &message.fields {
            let rule = WireRule::from_descriptor(field)?;
            if fields.insert(field.number, rule).is_some() {
                return Err(ArtifactError::Invalid("descriptor field number is duplicated"));
            }
        }
        Ok(Self { fields })
    }

    pub(crate) fn accepts(&self, number: u32, wire: u8) -> bool {
        self.fields.get(&number).is_none_or(|rule| rule.accepts(wire))
    }
}

#[derive(Debug, Clone, Copy)]
struct WireRule {
    primary: u8,
    packed: bool,
}

impl WireRule {
    fn from_descriptor(field: &DescriptorField) -> Result<Self, ArtifactError> {
        let (primary, packable) = match field.field_type {
            1 | 6 | 16 => (1, true),
            2 | 7 | 15 => (5, true),
            3..=5 | 8 | 13 | 14 | 17 | 18 => (0, true),
            9 | 11 | 12 => (2, false),
            _ => return Err(ArtifactError::Invalid("descriptor field type is unsupported")),
        };
        Ok(Self { primary, packed: field.label == 3 && packable })
    }

    const fn accepts(self, wire: u8) -> bool {
        wire == self.primary || (self.packed && wire == 2)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;

    #[test]
    fn load_rejects_unknown_schema_version_without_needing_private_descriptors() {
        let path = fixture_path("version");
        fs::write(&path, br#"{"schema_version":99,"api_map":{},"descriptors":{"messages":[]}}"#)
            .expect("fixture should write");

        let error = RuntimeArtifact::load(&path).expect_err("version should be rejected");
        fs::remove_file(path).expect("fixture should be removed");

        assert!(matches!(
            error,
            ArtifactError::UnsupportedSchemaVersion { actual: 99, expected: 1 }
        ));
    }

    #[test]
    fn load_rejects_incomplete_descriptor_artifact() {
        let path = fixture_path("incomplete");
        fs::write(&path, br#"{"schema_version":1,"api_map":{},"descriptors":{"messages":[]}}"#)
            .expect("fixture should write");

        let error = RuntimeArtifact::load(&path).expect_err("artifact should be incomplete");
        fs::remove_file(path).expect("fixture should be removed");

        assert!(matches!(error, ArtifactError::Invalid("required descriptor message is missing")));
    }

    fn fixture_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock should be valid")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("rokbattles-relay-artifact-{name}-{}-{nonce}.json", std::process::id()))
    }
}
