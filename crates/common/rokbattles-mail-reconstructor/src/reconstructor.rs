//! Public reconstruction API and assembly of the persistent mail envelope.
//!
//! Body adapters own category-specific transformations. This module supplies
//! connection fallbacks, preserves envelope fields, and encodes the assembled mail.

use std::path::Path;

use rokbattles_mail_registry::detect_mail_type;
use serde_json::{Map, Value};

use crate::{
    ReconstructionError,
    artifact::MailSchema,
    entity::{MailEntity, required_text},
    value::{decode_flags, decode_info, inflate_mail_body},
};

const MAX_MAIL_BYTES: usize = 25 * 1024 * 1024;
const UNREAD_STATUS: u64 = 0;

/// Connection metadata available when reconstructing a mail entry.
///
/// Only `server_id` is currently used as a fallback; `player_id` does not
/// replace the entry's receiver or receiver information.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconstructionContext {
    /// Server ID observed during login, used when the entry omits it or sends zero.
    ///
    /// A context value of zero is accepted as supplied.
    pub server_id: Option<i32>,
    /// Player ID observed during login; currently unused during reconstruction.
    pub player_id: Option<i64>,
}

/// A persistent mail reconstructed from one protobuf `MailEntity`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReconstructedMail {
    /// Persistent mail ID.
    pub id: String,
    /// Canonical category returned by the mail registry.
    ///
    /// May be more specific than the stored root type, such as a System or
    /// Alliance subcategory.
    pub mail_type: String,
    /// Complete encoded bytes for a `Persistent.Mail.<id>` file.
    ///
    /// The caller is responsible for storing or forwarding these bytes.
    pub bytes: Vec<u8>,
}

/// Reusable mail reconstructor backed by runtime protocol descriptors.
///
/// Loading validates the envelope descriptors and retains body descriptors for
/// later decoding. Calls to [`Self::reconstruct`] borrow this state and return
/// independent, owned results.
#[derive(Debug)]
pub struct MailReconstructor {
    pub(crate) schema: MailSchema,
    max_mail_bytes: usize,
}

impl MailReconstructor {
    /// Loads a version 1 JSON protocol artifact from `path`.
    ///
    /// Reads at most 32 MiB plus one byte to detect oversized files. Required
    /// `MailEntity` and `MailReportAttack` fields are resolved by name and checked
    /// for the expected descriptor type. Body descriptors are looked up when
    /// their mail category is reconstructed.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, exceeds 32 MiB, fails JSON
    /// deserialization, uses an unsupported schema version, or has missing,
    /// duplicated, or incompatible required descriptors.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ReconstructionError> {
        Ok(Self { schema: MailSchema::load(path.as_ref())?, max_mail_bytes: MAX_MAIL_BYTES })
    }

    #[cfg(test)]
    pub(crate) fn synthetic() -> Self {
        Self { schema: crate::artifact::synthetic_schema(), max_mail_bytes: MAX_MAIL_BYTES }
    }

    /// Reconstructs one raw protobuf `MailEntity` into persistent file bytes.
    ///
    /// `entry` must contain just the protobuf message, without transport framing.
    /// A compression tag of 1 selects zlib decoding of the primary body and
    /// requires an exact declared output length. Other tags leave the body as
    /// supplied. The entry and declared inflated body are limited to 25 MiB.
    ///
    /// A nonzero entry server ID takes precedence over `context.server_id`.
    /// Mail ID, mail type, and primary body must be nonempty. Reconstruction
    /// returns no partial result and does not run a mail processor.
    ///
    /// # Errors
    ///
    /// Returns an error for oversized entries, invalid wire fields or body
    /// data, failed decompression, missing required values or descriptors,
    /// unsupported categories, or values the persistent encoder cannot encode.
    /// See [`ReconstructionError`] for individual failure cases.
    pub fn reconstruct(
        &self,
        entry: &[u8],
        context: ReconstructionContext,
    ) -> Result<ReconstructedMail, ReconstructionError> {
        if entry.len() > self.max_mail_bytes {
            return Err(ReconstructionError::MailTooLarge { max: self.max_mail_bytes });
        }
        let entity = MailEntity::decode(entry, &self.schema)?;
        self.reconstruct_entity(entity, context)
    }

    fn reconstruct_entity(
        &self,
        entity: MailEntity<'_>,
        context: ReconstructionContext,
    ) -> Result<ReconstructedMail, ReconstructionError> {
        let body_bytes = if entity.compression_tag == 1 {
            inflate_mail_body(entity.body, entity.original_length, self.max_mail_bytes)?
        } else {
            entity.body.to_vec()
        };
        // The network Battle2 label uses the persistent Battle representation.
        let normalized_type =
            if entity.mail_type == "Battle2" { "Battle" } else { entity.mail_type.as_str() };
        let body = self.reconstruct_body(normalized_type, &body_bytes, &entity.attack_bodies)?;
        let attachments = self.reconstruct_attachments(&entity.attachments)?;

        let server_id = match entity.server_id {
            Some(server_id) if server_id != 0 => server_id,
            _ => context.server_id.ok_or(ReconstructionError::MissingServerId)?,
        };
        let id = required_text(entity.mail_id, "MailId")?.to_string();
        let mut value = Map::new();
        value.insert("id".to_string(), Value::String(id.clone()));
        value.insert("sender".to_string(), Value::String(entity.sender.to_string()));
        value.insert("senderInfo".to_string(), decode_info(entity.sender_info));
        value.insert("receiver".to_string(), Value::String(entity.receiver.to_string()));
        value.insert("receiverInfo".to_string(), decode_info(entity.receiver_info));
        value.insert("title".to_string(), Value::String(entity.title.to_string()));
        value.insert("body".to_string(), body);
        value.insert("unread".to_string(), Value::Bool(entity.status == UNREAD_STATUS));
        value.insert("box".to_string(), Value::String(entity.box_name.to_string()));
        value.insert("type".to_string(), Value::String(normalized_type.to_string()));
        value.insert("time".to_string(), Value::Number(entity.timestamp.into()));
        value.insert("mailScene".to_string(), Value::Number(i64::from(entity.scene).into()));
        value.insert("serverId".to_string(), Value::Number(i64::from(server_id).into()));
        value.insert("attachments".to_string(), Value::Array(attachments));
        value.insert("prevBox".to_string(), Value::String(entity.previous_box.to_string()));
        // Addition is opaque bytes on the wire but a string in persistent mail.
        // Invalid UTF-8 is replaced here; envelope string fields reject it.
        value.insert(
            "addition".to_string(),
            Value::String(String::from_utf8_lossy(entity.addition).into_owned()),
        );
        value.insert("flaglist".to_string(), decode_flags(entity.flag_list));
        value.insert("senderTradeTime".to_string(), Value::Number(entity.sender_trade_time.into()));
        value.insert("holdTag".to_string(), Value::Number(i64::from(entity.hold_tag).into()));
        value.insert(
            "reportMerge".to_string(),
            Value::Number(i64::from(entity.report_merge).into()),
        );
        value.insert("preMailId".to_string(), Value::String(entity.previous_mail_id.to_string()));
        value.insert("starMark".to_string(), Value::Number(i64::from(entity.star_level).into()));

        let value = Value::Object(value);
        // Broad System and Alliance labels need a supported body subtype.
        // Detection selects a category; it does not validate processor inputs.
        let mail_type = detect_mail_type(&value)
            .ok_or_else(|| ReconstructionError::UnsupportedMailType(normalized_type.to_string()))?;
        let bytes = rokbattles_mail_encoder::encode(&value)
            .map_err(ReconstructionError::PersistentEncoding)?;

        Ok(ReconstructedMail { id, mail_type: mail_type.to_string(), bytes })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::ZlibEncoder};

    use super::*;

    fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
        while value >= 0x80 {
            output.push((value as u8) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    fn push_varint(output: &mut Vec<u8>, number: u32, value: u64) {
        encode_varint(u64::from(number) << 3, output);
        encode_varint(value, output);
    }

    fn push_bytes(output: &mut Vec<u8>, number: u32, value: &[u8]) {
        encode_varint((u64::from(number) << 3) | 2, output);
        encode_varint(value.len() as u64, output);
        output.extend_from_slice(value);
    }

    fn synthetic_battle(reconstructor: &MailReconstructor, compressed: bool) -> Vec<u8> {
        let body = br#"{"Attacks":{},"Id":123}"#;
        let schema = &reconstructor.schema;
        let mut output = Vec::new();
        push_bytes(&mut output, schema.mail_id, b"123");
        push_bytes(&mut output, schema.sender, b"system");
        push_bytes(&mut output, schema.sender_info, b"system");
        push_bytes(&mut output, schema.receiver, b"player_1");
        push_bytes(&mut output, schema.mail_type, b"Battle2");
        push_bytes(&mut output, schema.box_name, b"Archive");
        push_varint(&mut output, schema.timestamp, 42);
        if compressed {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(body).expect("body should compress");
            let compressed_body = encoder.finish().expect("compression should finish");
            push_bytes(&mut output, schema.body, &compressed_body);
            push_varint(&mut output, schema.compression_tag, 1);
            push_varint(&mut output, schema.original_length, body.len() as u64);
        } else {
            push_bytes(&mut output, schema.body, body);
        }
        output
    }

    fn decode_reconstructed(mail: &ReconstructedMail) -> Value {
        rokbattles_mail_decoder::decode(&mail.bytes).expect("reconstructed file should decode")
    }

    #[test]
    fn reconstructs_battle_with_connection_server_fallback() {
        let reconstructor = MailReconstructor::synthetic();
        let raw = synthetic_battle(&reconstructor, false);

        let mail = reconstructor
            .reconstruct(
                &raw,
                ReconstructionContext { server_id: Some(1804), ..Default::default() },
            )
            .expect("mail should reconstruct");

        assert_eq!(mail.id, "123");
        assert_eq!(mail.mail_type, "Battle");
        let value = decode_reconstructed(&mail);
        assert_eq!(value["type"], "Battle");
        assert_eq!(value["serverId"], 1804);
        assert_eq!(value["body"]["content"]["Id"], 123);
    }

    #[test]
    fn prefers_nonzero_entity_server_id_over_connection_context() {
        let reconstructor = MailReconstructor::synthetic();
        let mut raw = synthetic_battle(&reconstructor, false);
        push_varint(&mut raw, reconstructor.schema.server_id, 1900);

        let mail = reconstructor
            .reconstruct(
                &raw,
                ReconstructionContext { server_id: Some(1804), ..Default::default() },
            )
            .expect("mail should reconstruct");

        assert_eq!(decode_reconstructed(&mail)["serverId"], 1900);
    }

    #[test]
    fn zero_entity_server_id_uses_connection_context() {
        let reconstructor = MailReconstructor::synthetic();
        let mut raw = synthetic_battle(&reconstructor, false);
        push_varint(&mut raw, reconstructor.schema.server_id, 0);

        let mail = reconstructor
            .reconstruct(
                &raw,
                ReconstructionContext { server_id: Some(1804), ..Default::default() },
            )
            .expect("mail should reconstruct");

        assert_eq!(decode_reconstructed(&mail)["serverId"], 1804);
    }

    #[test]
    fn reconstructs_mail_level_zlib_body() {
        let reconstructor = MailReconstructor::synthetic();
        let raw = synthetic_battle(&reconstructor, true);

        let mail = reconstructor
            .reconstruct(
                &raw,
                ReconstructionContext { server_id: Some(1804), ..Default::default() },
            )
            .expect("compressed mail should reconstruct");

        assert_eq!(decode_reconstructed(&mail)["body"]["content"]["Id"], 123);
    }

    #[test]
    fn rejects_unknown_mail_type() {
        let reconstructor = MailReconstructor::synthetic();
        let mut raw = synthetic_battle(&reconstructor, false);
        push_bytes(&mut raw, reconstructor.schema.mail_type, b"UnknownMail");

        let error = reconstructor
            .reconstruct(
                &raw,
                ReconstructionContext { server_id: Some(1804), ..Default::default() },
            )
            .expect_err("unverified type should fail");

        assert!(matches!(
            error,
            ReconstructionError::UnsupportedMailType(value) if value == "UnknownMail"
        ));
    }

    #[test]
    fn requires_server_id_from_entry_or_context() {
        let reconstructor = MailReconstructor::synthetic();
        let raw = synthetic_battle(&reconstructor, false);

        assert!(matches!(
            reconstructor.reconstruct(&raw, ReconstructionContext::default()),
            Err(ReconstructionError::MissingServerId)
        ));
    }

    #[test]
    fn rejects_entries_larger_than_configured_bound() {
        let mut reconstructor = MailReconstructor::synthetic();
        reconstructor.max_mail_bytes = 4;

        assert!(matches!(
            reconstructor.reconstruct(&[0; 5], ReconstructionContext::default()),
            Err(ReconstructionError::MailTooLarge { max: 4 })
        ));
    }
}
