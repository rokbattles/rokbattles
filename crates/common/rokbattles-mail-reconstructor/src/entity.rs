//! Borrowed decoding of the MailEntity protobuf envelope.
//!
//! Text and byte fields borrow the entry buffer; nested bodies stay encoded until
//! category reconstruction. Missing fields retain their initialized defaults,
//! repeated bodies append in wire order, and singular fields use the last value.

use crate::{
    ReconstructionError,
    artifact::MailSchema,
    protobuf::{FieldValue, fields},
};

/// Envelope values and encoded nested messages awaiting body reconstruction.
#[derive(Debug)]
pub(crate) struct MailEntity<'a> {
    pub mail_id: &'a str,
    pub sender: &'a str,
    pub sender_info: &'a str,
    pub receiver: &'a str,
    pub title: &'a str,
    pub body: &'a [u8],
    pub status: u64,
    pub box_name: &'a str,
    pub mail_type: String,
    pub timestamp: i64,
    pub receiver_info: &'a str,
    pub scene: i32,
    pub server_id: Option<i32>,
    pub attachments: Vec<&'a [u8]>,
    pub previous_box: &'a str,
    pub addition: &'a [u8],
    pub original_length: Option<i32>,
    pub compression_tag: i32,
    pub flag_list: &'a str,
    pub sender_trade_time: i64,
    pub hold_tag: i32,
    pub report_merge: i32,
    pub previous_mail_id: &'a str,
    pub star_level: i32,
    pub attack_bodies: Vec<&'a [u8]>,
}

impl<'a> MailEntity<'a> {
    /// Decodes known fields and requires a nonempty type and primary body.
    ///
    /// Mail ID and server fallback checks happen during final assembly. Unknown
    /// fields are ignored after their wire values have been read.
    pub(crate) fn decode(data: &'a [u8], schema: &MailSchema) -> Result<Self, ReconstructionError> {
        let mut entity = Self {
            mail_id: "",
            sender: "",
            sender_info: "",
            receiver: "",
            title: "",
            body: &[],
            status: 0,
            box_name: "",
            mail_type: String::new(),
            timestamp: 0,
            receiver_info: "",
            scene: 0,
            server_id: None,
            attachments: Vec::new(),
            previous_box: "",
            addition: &[],
            original_length: None,
            compression_tag: 0,
            flag_list: "",
            sender_trade_time: 0,
            hold_tag: 0,
            report_merge: 0,
            previous_mail_id: "",
            star_level: 0,
            attack_bodies: Vec::new(),
        };

        for field in fields(data) {
            let field = field?;
            match field.number {
                number if number == schema.mail_id => entity.mail_id = text(field.value)?,
                number if number == schema.sender => entity.sender = text(field.value)?,
                number if number == schema.sender_info => entity.sender_info = text(field.value)?,
                number if number == schema.receiver => entity.receiver = text(field.value)?,
                number if number == schema.title => entity.title = text(field.value)?,
                number if number == schema.body => entity.body = bytes(field.value)?,
                number if number == schema.status => entity.status = varint(field.value)?,
                number if number == schema.box_name => entity.box_name = text(field.value)?,
                number if number == schema.mail_type => {
                    entity.mail_type = text(field.value)?.to_string();
                }
                number if number == schema.timestamp => {
                    entity.timestamp = signed_i64(varint(field.value)?);
                }
                number if number == schema.receiver_info => {
                    entity.receiver_info = text(field.value)?;
                }
                number if number == schema.scene => {
                    entity.scene = to_i32(varint(field.value)?)?;
                }
                number if number == schema.server_id => {
                    entity.server_id = Some(to_i32(varint(field.value)?)?);
                }
                number if number == schema.attachments => {
                    entity.attachments.push(bytes(field.value)?);
                }
                number if number == schema.previous_box => {
                    entity.previous_box = text(field.value)?;
                }
                number if number == schema.addition => entity.addition = bytes(field.value)?,
                number if number == schema.original_length => {
                    entity.original_length = Some(to_i32(varint(field.value)?)?);
                }
                number if number == schema.compression_tag => {
                    entity.compression_tag = to_i32(varint(field.value)?)?;
                }
                number if number == schema.flag_list => entity.flag_list = text(field.value)?,
                number if number == schema.sender_trade_time => {
                    entity.sender_trade_time = signed_i64(varint(field.value)?);
                }
                number if number == schema.hold_tag => {
                    entity.hold_tag = to_i32(varint(field.value)?)?;
                }
                number if number == schema.report_merge => {
                    entity.report_merge = to_i32(varint(field.value)?)?;
                }
                number if number == schema.previous_mail_id => {
                    entity.previous_mail_id = text(field.value)?;
                }
                number if number == schema.star_level => {
                    entity.star_level = to_i32(varint(field.value)?)?;
                }
                number if number == schema.attack_bodies => {
                    entity.attack_bodies.push(bytes(field.value)?);
                }
                _ => {}
            }
        }

        required_text(&entity.mail_type, "Type")?;
        (!entity.body.is_empty()).then_some(()).ok_or(ReconstructionError::MissingField("Body"))?;
        Ok(entity)
    }
}

pub(crate) fn required_text<'a>(
    value: &'a str,
    field: &'static str,
) -> Result<&'a str, ReconstructionError> {
    (!value.is_empty()).then_some(value).ok_or(ReconstructionError::MissingField(field))
}

pub(crate) fn text(value: FieldValue<'_>) -> Result<&str, ReconstructionError> {
    std::str::from_utf8(bytes(value)?)
        .map_err(|_error| ReconstructionError::InvalidProtobuf("string field is not UTF-8"))
}

pub(crate) fn bytes(value: FieldValue<'_>) -> Result<&[u8], ReconstructionError> {
    match value {
        FieldValue::Bytes(value) => Ok(value),
        _ => Err(ReconstructionError::InvalidProtobuf(
            "field has an incompatible protobuf wire type",
        )),
    }
}

fn varint(value: FieldValue<'_>) -> Result<u64, ReconstructionError> {
    match value {
        FieldValue::Varint(value) => Ok(value),
        _ => Err(ReconstructionError::InvalidProtobuf(
            "field has an incompatible protobuf wire type",
        )),
    }
}

// These conversions preserve signed bit patterns rather than applying ZigZag.
// The i32 path accepts only values that first fit in u32.
fn to_i32(value: u64) -> Result<i32, ReconstructionError> {
    let narrowed = u32::try_from(value).map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
    Ok(i32::from_ne_bytes(narrowed.to_ne_bytes()))
}

fn signed_i64(value: u64) -> i64 {
    i64::from_ne_bytes(value.to_ne_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::synthetic_schema;

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

    #[test]
    fn decode_requires_mail_type_and_body() {
        let schema = synthetic_schema();
        let mut body_only = Vec::new();
        push_bytes(&mut body_only, schema.body, b"{}");
        assert!(matches!(
            MailEntity::decode(&body_only, &schema),
            Err(ReconstructionError::MissingField("Type"))
        ));

        let mut type_only = Vec::new();
        push_bytes(&mut type_only, schema.mail_type, b"Battle2");
        assert!(matches!(
            MailEntity::decode(&type_only, &schema),
            Err(ReconstructionError::MissingField("Body"))
        ));
    }

    #[test]
    fn decode_rejects_wrong_wire_type_for_known_field() {
        let schema = synthetic_schema();
        let mut data = Vec::new();
        push_varint(&mut data, schema.mail_type, 1);

        assert!(matches!(
            MailEntity::decode(&data, &schema),
            Err(ReconstructionError::InvalidProtobuf(
                "field has an incompatible protobuf wire type"
            ))
        ));
    }

    #[test]
    fn required_text_rejects_empty_values() {
        assert!(matches!(
            required_text("", "MailId"),
            Err(ReconstructionError::MissingField("MailId"))
        ));
        assert_eq!(required_text("123", "MailId").expect("value should be present"), "123");
    }

    #[test]
    fn scalar_helpers_reject_incompatible_wire_types() {
        assert!(matches!(
            text(FieldValue::Varint(1)),
            Err(ReconstructionError::InvalidProtobuf(_))
        ));
        assert!(matches!(
            varint(FieldValue::Bytes(b"1")),
            Err(ReconstructionError::InvalidProtobuf(_))
        ));
    }

    #[test]
    fn text_rejects_invalid_utf8() {
        assert!(matches!(
            text(FieldValue::Bytes(&[0xff])),
            Err(ReconstructionError::InvalidProtobuf("string field is not UTF-8"))
        ));
    }

    #[test]
    fn signed_integer_conversions_preserve_twos_complement() {
        assert_eq!(to_i32(u64::from(u32::MAX)).expect("u32 should fit"), -1);
        assert_eq!(signed_i64(u64::MAX), -1);
        assert!(matches!(
            to_i32(u64::from(u32::MAX) + 1),
            Err(ReconstructionError::IntegerOutOfRange)
        ));
    }
}
