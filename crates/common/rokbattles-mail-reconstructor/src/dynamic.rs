//! Descriptor-driven protobuf body decoding into JSON values.
//!
//! Field names retain artifact spelling. Unknown wire fields are skipped, repeated
//! values append in wire order, and later singular values replace earlier ones.
//! Bytes become base64 strings, enums remain numbers, and nonfinite floats fail.
//! Missing nested messages are omitted instead of recursively creating defaults.

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Map, Number, Value};

use crate::{
    ReconstructionError,
    artifact::{DescriptorPool, DynamicField, DynamicMessage},
    protobuf::{FieldValue, fields},
};

const LABEL_REPEATED: u8 = 3;

/// Decodes a named message, initializing scalar and repeated field defaults.
///
/// Descriptor references are resolved as nested messages are encountered.
pub(crate) fn decode_message(
    data: &[u8],
    name: &str,
    descriptors: &DescriptorPool,
) -> Result<Value, ReconstructionError> {
    let descriptor = descriptors.message(name)?;
    let mut output = defaults(descriptor);

    for encoded in fields(data) {
        let encoded = encoded?;
        let Some(field) = descriptor.fields.iter().find(|field| field.number == encoded.number)
        else {
            continue;
        };
        if field.label == LABEL_REPEATED {
            let values = decode_repeated(encoded.value, field, descriptors)?;
            let destination = output.get_mut(&field.name).and_then(Value::as_array_mut).ok_or(
                ReconstructionError::InvalidArtifact(
                    "repeated descriptor field did not initialize as an array",
                ),
            )?;
            destination.extend(values);
        } else {
            output.insert(field.name.clone(), decode_value(encoded.value, field, descriptors)?);
        }
    }

    Ok(Value::Object(output))
}

fn defaults(descriptor: &DynamicMessage) -> Map<String, Value> {
    descriptor
        .fields
        .iter()
        .filter_map(|field| {
            let value = if field.label == LABEL_REPEATED {
                Value::Array(Vec::new())
            } else {
                scalar_default(field.field_type)?
            };
            Some((field.name.clone(), value))
        })
        .collect()
}

/// Supplies zero or empty scalar values; message fields have no default here.
fn scalar_default(field_type: u8) -> Option<Value> {
    match field_type {
        1 | 2 => Number::from_f64(0.0).map(Value::Number),
        // An absent bool uses numeric zero here; a present bool decodes to JSON bool.
        3..=8 | 13..=18 => Some(Value::Number(0.into())),
        9 | 12 => Some(Value::String(String::new())),
        11 => None,
        _ => None,
    }
}

fn decode_repeated(
    value: FieldValue<'_>,
    field: &DynamicField,
    descriptors: &DescriptorPool,
) -> Result<Vec<Value>, ReconstructionError> {
    // Numeric repeated fields may arrive packed or as individual occurrences.
    if let FieldValue::Bytes(packed) = value
        && is_packable(field.field_type)
    {
        return decode_packed(packed, field.field_type);
    }
    Ok(vec![decode_value(value, field, descriptors)?])
}

fn decode_value(
    value: FieldValue<'_>,
    field: &DynamicField,
    descriptors: &DescriptorPool,
) -> Result<Value, ReconstructionError> {
    // These are protobuf descriptor type codes, distinct from wire type tags.
    match field.field_type {
        1 => match value {
            FieldValue::Fixed64(bits) => finite_number(f64::from_bits(bits)),
            _ => wrong_wire(),
        },
        2 => match value {
            FieldValue::Fixed32(bits) => finite_number(f64::from(f32::from_bits(bits))),
            _ => wrong_wire(),
        },
        3 => match value {
            FieldValue::Varint(value) => Ok(Value::Number(signed_i64(value).into())),
            _ => wrong_wire(),
        },
        4 => match value {
            FieldValue::Varint(value) => Ok(Value::Number(value.into())),
            _ => wrong_wire(),
        },
        5 | 14 => match value {
            FieldValue::Varint(value) => Ok(Value::Number(i64::from(to_i32(value)?).into())),
            _ => wrong_wire(),
        },
        6 => match value {
            FieldValue::Fixed64(value) => Ok(Value::Number(value.into())),
            _ => wrong_wire(),
        },
        7 => match value {
            FieldValue::Fixed32(value) => Ok(Value::Number(u64::from(value).into())),
            _ => wrong_wire(),
        },
        8 => match value {
            FieldValue::Varint(value) => Ok(Value::Bool(value != 0)),
            _ => wrong_wire(),
        },
        9 => match value {
            FieldValue::Bytes(value) => {
                std::str::from_utf8(value).map(|value| Value::String(value.to_string())).map_err(
                    |_error| ReconstructionError::InvalidProtobuf("string field is not UTF-8"),
                )
            }
            _ => wrong_wire(),
        },
        11 => match value {
            FieldValue::Bytes(value) => decode_message(value, &field.type_name, descriptors),
            _ => wrong_wire(),
        },
        12 => match value {
            FieldValue::Bytes(value) => Ok(Value::String(STANDARD.encode(value))),
            _ => wrong_wire(),
        },
        13 => match value {
            FieldValue::Varint(value) => {
                let value = u32::try_from(value)
                    .map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
                Ok(Value::Number(u64::from(value).into()))
            }
            _ => wrong_wire(),
        },
        15 => match value {
            FieldValue::Fixed32(value) => {
                Ok(Value::Number(i64::from(i32::from_le_bytes(value.to_le_bytes())).into()))
            }
            _ => wrong_wire(),
        },
        16 => match value {
            FieldValue::Fixed64(value) => {
                Ok(Value::Number(i64::from_le_bytes(value.to_le_bytes()).into()))
            }
            _ => wrong_wire(),
        },
        17 => match value {
            FieldValue::Varint(value) => {
                let value = u32::try_from(value)
                    .map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
                Ok(Value::Number(i64::from(zigzag_i32(value)).into()))
            }
            _ => wrong_wire(),
        },
        18 => match value {
            FieldValue::Varint(value) => Ok(Value::Number(zigzag_i64(value).into())),
            _ => wrong_wire(),
        },
        _ => Err(ReconstructionError::InvalidArtifact("descriptor field type is unsupported")),
    }
}

/// Decodes a tagless sequence using the repeated field's scalar type.
fn decode_packed(data: &[u8], field_type: u8) -> Result<Vec<Value>, ReconstructionError> {
    match field_type {
        1 | 6 | 16 => {
            let (chunks, remainder) = data.as_chunks::<8>();
            if !remainder.is_empty() {
                return Err(ReconstructionError::InvalidProtobuf("truncated packed fixed64 field"));
            }
            chunks
                .iter()
                .map(|bytes| match field_type {
                    1 => finite_number(f64::from_le_bytes(*bytes)),
                    6 => Ok(Value::Number(u64::from_le_bytes(*bytes).into())),
                    _ => Ok(Value::Number(i64::from_le_bytes(*bytes).into())),
                })
                .collect()
        }
        2 | 7 | 15 => {
            let (chunks, remainder) = data.as_chunks::<4>();
            if !remainder.is_empty() {
                return Err(ReconstructionError::InvalidProtobuf("truncated packed fixed32 field"));
            }
            chunks
                .iter()
                .map(|bytes| match field_type {
                    2 => finite_number(f64::from(f32::from_le_bytes(*bytes))),
                    7 => Ok(Value::Number(u64::from(u32::from_le_bytes(*bytes)).into())),
                    _ => Ok(Value::Number(i64::from(i32::from_le_bytes(*bytes)).into())),
                })
                .collect()
        }
        3..=5 | 8 | 13 | 14 | 17 | 18 => {
            let mut values = Vec::new();
            let mut remaining = data;
            while !remaining.is_empty() {
                let (value, used) = read_varint(remaining)?;
                remaining = remaining
                    .get(used..)
                    .ok_or(ReconstructionError::InvalidProtobuf("truncated packed varint field"))?;
                let decoded = match field_type {
                    3 => Value::Number(signed_i64(value).into()),
                    4 => Value::Number(value.into()),
                    5 | 14 => Value::Number(i64::from(to_i32(value)?).into()),
                    8 => Value::Bool(value != 0),
                    13 => Value::Number(
                        u64::from(
                            u32::try_from(value)
                                .map_err(|_error| ReconstructionError::IntegerOutOfRange)?,
                        )
                        .into(),
                    ),
                    17 => Value::Number(
                        i64::from(zigzag_i32(
                            u32::try_from(value)
                                .map_err(|_error| ReconstructionError::IntegerOutOfRange)?,
                        ))
                        .into(),
                    ),
                    18 => Value::Number(zigzag_i64(value).into()),
                    _ => {
                        return Err(ReconstructionError::InvalidArtifact(
                            "packed descriptor field type is unsupported",
                        ));
                    }
                };
                values.push(decoded);
            }
            Ok(values)
        }
        _ => {
            Err(ReconstructionError::InvalidArtifact("packed descriptor field type is unsupported"))
        }
    }
}

fn read_varint(data: &[u8]) -> Result<(u64, usize), ReconstructionError> {
    let mut value = 0u64;
    for (index, byte) in data.iter().take(10).enumerate() {
        let shift = u32::try_from(index.saturating_mul(7))
            .map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
    }
    Err(ReconstructionError::InvalidProtobuf("packed protobuf varint exceeded 64 bits"))
}

fn is_packable(field_type: u8) -> bool {
    matches!(field_type, 1..=8 | 13..=18)
}

fn finite_number(value: f64) -> Result<Value, ReconstructionError> {
    Number::from_f64(value)
        .map(Value::Number)
        .ok_or(ReconstructionError::InvalidProtobuf("floating-point field is not finite"))
}

fn wrong_wire<T>() -> Result<T, ReconstructionError> {
    Err(ReconstructionError::InvalidProtobuf("field has an incompatible protobuf wire type"))
}

fn to_i32(value: u64) -> Result<i32, ReconstructionError> {
    let narrowed = u32::try_from(value).map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
    Ok(i32::from_ne_bytes(narrowed.to_ne_bytes()))
}

fn signed_i64(value: u64) -> i64 {
    i64::from_ne_bytes(value.to_ne_bytes())
}

fn zigzag_i32(value: u32) -> i32 {
    ((value >> 1) as i32) ^ -((value & 1) as i32)
}

fn zigzag_i64(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}
