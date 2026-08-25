use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::ReconstructionError;

const CURRENT_SCHEMA_VERSION: u32 = 1;
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct MailSchema {
    pub descriptors: DescriptorPool,
    pub mail_id: u32,
    pub sender: u32,
    pub sender_info: u32,
    pub receiver: u32,
    pub title: u32,
    pub body: u32,
    pub status: u32,
    pub box_name: u32,
    pub mail_type: u32,
    pub timestamp: u32,
    pub receiver_info: u32,
    pub scene: u32,
    pub server_id: u32,
    pub attachments: u32,
    pub previous_box: u32,
    pub addition: u32,
    pub original_length: u32,
    pub compression_tag: u32,
    pub flag_list: u32,
    pub sender_trade_time: u32,
    pub hold_tag: u32,
    pub report_merge: u32,
    pub previous_mail_id: u32,
    pub star_level: u32,
    pub attack_bodies: u32,
    pub attack_name: u32,
    pub attack_body: u32,
}

impl MailSchema {
    pub(crate) fn load(path: &Path) -> Result<Self, ReconstructionError> {
        let raw = read_bounded(path)?;
        Self::from_slice(&raw)
    }

    fn from_slice(raw: &[u8]) -> Result<Self, ReconstructionError> {
        let artifact: ArtifactFile =
            serde_json::from_slice(raw).map_err(ReconstructionError::InvalidArtifactJson)?;
        if artifact.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ReconstructionError::UnsupportedArtifactVersion {
                actual: artifact.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }

        let messages = artifact
            .descriptors
            .messages
            .iter()
            .map(|message| (normalize_name(&message.name), message))
            .collect::<HashMap<_, _>>();
        let mail = required_message(&messages, "MailEntity")?;
        let attack = required_message(&messages, "MailReportAttack")?;

        let descriptors = DescriptorPool::from_messages(&artifact.descriptors.messages)?;
        Ok(Self {
            descriptors,
            mail_id: mail.required_field("MailId", FieldType::String)?,
            sender: mail.required_field("Sender", FieldType::String)?,
            sender_info: mail.required_field("SenderInfo", FieldType::String)?,
            receiver: mail.required_field("Receiver", FieldType::String)?,
            title: mail.required_field("Title", FieldType::String)?,
            body: mail.required_field("Body", FieldType::Bytes)?,
            status: mail.required_field("Status", FieldType::Enum)?,
            box_name: mail.required_field("Box", FieldType::String)?,
            mail_type: mail.required_field("Type", FieldType::String)?,
            timestamp: mail.required_field("Timestamp", FieldType::Int64)?,
            receiver_info: mail.required_field("ReceiverInfo", FieldType::String)?,
            scene: mail.required_field("Scene", FieldType::Enum)?,
            server_id: mail.required_field("ServerId", FieldType::Int32)?,
            attachments: mail.required_field("Attachments", FieldType::Message)?,
            previous_box: mail.required_field("PrevBox", FieldType::String)?,
            addition: mail.required_field("Addtion", FieldType::Bytes)?,
            original_length: mail.required_field("OriLen", FieldType::Int32)?,
            compression_tag: mail.required_field("Tag", FieldType::Int32)?,
            flag_list: mail.required_field("FlagList", FieldType::String)?,
            sender_trade_time: mail.required_field("SenderTradeTime", FieldType::Int64)?,
            hold_tag: mail.required_field("HoldTag", FieldType::Int32)?,
            report_merge: mail.required_field("ReportMerge", FieldType::Int32)?,
            previous_mail_id: mail.required_field("PreMailId", FieldType::String)?,
            star_level: mail.required_field("StarLevel", FieldType::Int32)?,
            attack_bodies: mail.required_field("AttacksBody", FieldType::Message)?,
            attack_name: attack.required_field("Attack", FieldType::String)?,
            attack_body: attack.required_field("Body", FieldType::Bytes)?,
        })
    }
}

#[derive(Debug)]
pub(crate) struct DescriptorPool {
    messages: HashMap<String, DynamicMessage>,
}

impl DescriptorPool {
    fn from_messages(messages: &[DescriptorMessage]) -> Result<Self, ReconstructionError> {
        let mut indexed = HashMap::with_capacity(messages.len());
        for message in messages {
            let name = normalize_name(&message.name).to_string();
            let dynamic = DynamicMessage {
                fields: message
                    .fields
                    .iter()
                    .map(|field| DynamicField {
                        name: field.name.clone(),
                        number: field.number,
                        label: field.label,
                        field_type: field.field_type,
                        type_name: normalize_name(&field.type_name).to_string(),
                    })
                    .collect(),
            };
            if indexed.insert(name, dynamic).is_some() {
                return Err(ReconstructionError::InvalidArtifact(
                    "descriptor message name is duplicated",
                ));
            }
        }
        Ok(Self { messages: indexed })
    }

    pub(crate) fn message(&self, name: &str) -> Result<&DynamicMessage, ReconstructionError> {
        self.messages
            .get(normalize_name(name))
            .ok_or(ReconstructionError::InvalidArtifact("body descriptor message is missing"))
    }
}

#[derive(Debug)]
pub(crate) struct DynamicMessage {
    pub fields: Vec<DynamicField>,
}

#[derive(Debug)]
pub(crate) struct DynamicField {
    pub name: String,
    pub number: u32,
    pub label: u8,
    pub field_type: u8,
    pub type_name: String,
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ReconstructionError> {
    let file = File::open(path)
        .map_err(|source| ReconstructionError::ReadArtifact { path: path.to_path_buf(), source })?;
    let mut bytes = Vec::new();
    file.take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| ReconstructionError::ReadArtifact { path: path.to_path_buf(), source })?;
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(ReconstructionError::ArtifactTooLarge {
            path: path.to_path_buf(),
            max: MAX_ARTIFACT_BYTES,
        });
    }
    Ok(bytes)
}

fn required_message<'a>(
    messages: &'a HashMap<&str, &'a DescriptorMessage>,
    name: &'static str,
) -> Result<&'a DescriptorMessage, ReconstructionError> {
    messages
        .get(name)
        .copied()
        .ok_or(ReconstructionError::InvalidArtifact("required descriptor message is missing"))
}

fn normalize_name(name: &str) -> &str {
    name.trim_start_matches('.')
}

#[derive(Debug, Deserialize)]
struct ArtifactFile {
    schema_version: u32,
    descriptors: DescriptorArtifact,
}

#[derive(Debug, Deserialize)]
struct DescriptorArtifact {
    messages: Vec<DescriptorMessage>,
}

#[derive(Debug, Deserialize)]
struct DescriptorMessage {
    name: String,
    fields: Vec<DescriptorField>,
}

impl DescriptorMessage {
    fn required_field(
        &self,
        name: &'static str,
        field_type: FieldType,
    ) -> Result<u32, ReconstructionError> {
        let matching = self
            .fields
            .iter()
            .filter(|field| field.name.eq_ignore_ascii_case(name))
            .collect::<Vec<_>>();
        let [field] = matching.as_slice() else {
            return Err(ReconstructionError::InvalidArtifact(
                "required descriptor field is missing or duplicated",
            ));
        };
        if field.field_type != field_type.code() {
            return Err(ReconstructionError::InvalidArtifact(
                "required descriptor field has an incompatible type",
            ));
        }
        Ok(field.number)
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

#[derive(Debug, Clone, Copy)]
enum FieldType {
    Int64,
    Int32,
    String,
    Message,
    Bytes,
    Enum,
}

impl FieldType {
    const fn code(self) -> u8 {
        match self {
            Self::Int64 => 3,
            Self::Int32 => 5,
            Self::String => 9,
            Self::Message => 11,
            Self::Bytes => 12,
            Self::Enum => 14,
        }
    }
}

impl From<io::Error> for ReconstructionError {
    fn from(source: io::Error) -> Self {
        Self::ReadArtifact { path: PathBuf::new(), source }
    }
}

#[cfg(test)]
pub(crate) use tests::synthetic_schema;

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn synthetic_schema() -> MailSchema {
        let descriptors = DescriptorPool {
            messages: [
                message("DuelMailReport", vec![field("FightId", 1, 1, 3, "")]),
                message("PosInfo", vec![field("X", 1, 1, 2, ""), field("Y", 2, 1, 2, "")]),
                message(
                    "EliteBarReportInfo",
                    vec![
                        field("Pos", 1, 1, 11, "PosInfo"),
                        field("NpcType", 2, 1, 5, ""),
                        field("Level", 3, 1, 5, ""),
                        field("Infos", 4, 3, 11, "ReportInfo"),
                    ],
                ),
                message(
                    "EventMemeberLootInfo",
                    vec![
                        field("Title", 1, 1, 9, ""),
                        field("SubTitleParam", 2, 1, 9, ""),
                        field("EventName", 3, 1, 9, ""),
                        field("Body", 4, 1, 9, ""),
                        field("Infos", 5, 3, 11, "ReportInfo"),
                    ],
                ),
                message(
                    "MailRss",
                    vec![
                        field("ResType", 1, 1, 5, ""),
                        field("ResValue", 2, 1, 1, ""),
                        field("Level", 3, 1, 5, ""),
                        field("Time", 4, 1, 3, ""),
                        field("Pos", 5, 1, 11, "PosInfo"),
                    ],
                ),
                message(
                    "MailSys",
                    vec![
                        field("Type", 1, 1, 5, ""),
                        field("Param", 2, 1, 5, ""),
                        field("Kvs", 3, 1, 9, ""),
                    ],
                ),
                message(
                    "MailAttachment",
                    vec![
                        field("Id", 1, 1, 3, ""),
                        field("Status", 2, 1, 5, ""),
                        field("Data", 3, 3, 11, "Loot"),
                    ],
                ),
                message("ReportInfo", Vec::new()),
                message(
                    "Loot",
                    vec![
                        field("Type", 1, 1, 5, ""),
                        field("SubType", 2, 1, 5, ""),
                        field("Value", 3, 1, 3, ""),
                    ],
                ),
            ]
            .into_iter()
            .collect(),
        };

        MailSchema {
            descriptors,
            mail_id: 1,
            sender: 2,
            sender_info: 3,
            receiver: 4,
            title: 5,
            body: 6,
            status: 7,
            box_name: 8,
            mail_type: 9,
            timestamp: 10,
            receiver_info: 11,
            scene: 12,
            server_id: 13,
            attachments: 14,
            previous_box: 15,
            addition: 16,
            original_length: 17,
            compression_tag: 18,
            flag_list: 19,
            sender_trade_time: 20,
            hold_tag: 21,
            report_merge: 22,
            previous_mail_id: 23,
            star_level: 24,
            attack_bodies: 25,
            attack_name: 1,
            attack_body: 2,
        }
    }

    fn message(name: &str, fields: Vec<DynamicField>) -> (String, DynamicMessage) {
        (name.to_string(), DynamicMessage { fields })
    }

    fn field(name: &str, number: u32, label: u8, field_type: u8, type_name: &str) -> DynamicField {
        DynamicField {
            name: name.to_string(),
            number,
            label,
            field_type,
            type_name: type_name.to_string(),
        }
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(matches!(
            MailSchema::from_slice(b"{"),
            Err(ReconstructionError::InvalidArtifactJson(_))
        ));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let artifact = br#"{"schema_version":2,"descriptors":{"messages":[]}}"#;

        assert!(matches!(
            MailSchema::from_slice(artifact),
            Err(ReconstructionError::UnsupportedArtifactVersion {
                actual: 2,
                expected: CURRENT_SCHEMA_VERSION
            })
        ));
    }

    #[test]
    fn rejects_missing_required_messages() {
        let artifact = br#"{"schema_version":1,"descriptors":{"messages":[]}}"#;

        assert!(matches!(
            MailSchema::from_slice(artifact),
            Err(ReconstructionError::InvalidArtifact("required descriptor message is missing"))
        ));
    }

    #[test]
    fn descriptor_pool_rejects_duplicate_message_names() {
        let messages = [
            DescriptorMessage { name: "Mail".to_string(), fields: Vec::new() },
            DescriptorMessage { name: ".Mail".to_string(), fields: Vec::new() },
        ];

        assert!(matches!(
            DescriptorPool::from_messages(&messages),
            Err(ReconstructionError::InvalidArtifact("descriptor message name is duplicated"))
        ));
    }
}
