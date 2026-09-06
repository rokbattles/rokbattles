//! JavaScript bindings for the Rust reader.
//!
//! Exported classes own Rust allocations. JavaScript callers release them with
//! `free()` after use. Decoded values are JavaScript-owned
//! copies and remain valid after releasing the result wrapper.

use wasm_bindgen::prelude::*;

use crate::{Error, HEADER_LEN, ReadLimits, Reader, schema};

#[wasm_bindgen(typescript_custom_section)]
const VALUE_TYPES: &str = include_str!("wasm-types.ts");

/// Reader for browsers and Node.js, exported as the JavaScript class `Reader`.
#[wasm_bindgen(js_name = Reader)]
pub struct WasmReader {
    reader: Reader,
    max_payload_len: u32,
}

/// Decoded metadata and a JavaScript value selected by the file's schema.
#[wasm_bindgen]
pub struct DecodedValue {
    schema_id: u16,
    value: JsValue,
}

#[wasm_bindgen]
impl DecodedValue {
    /// Schema ID read from the header.
    #[wasm_bindgen(getter, js_name = schemaId)]
    pub fn schema_id(&self) -> u16 {
        self.schema_id
    }

    /// Returns the decoded JavaScript value.
    ///
    /// Bytes become `Uint8Array`, text becomes `string`, and JSON becomes
    /// JavaScript objects and arrays. JSON integers become `BigInt`, including
    /// small integers; floating-point values become `Number`. Territory schema
    /// values use JavaScript numbers and objects, with province cells exported
    /// as `Uint8Array` and blocked province IDs as arrays.
    #[wasm_bindgen(getter, unchecked_return_type = "ContainerValue")]
    pub fn value(&self) -> JsValue {
        self.value.clone()
    }
}

#[wasm_bindgen(js_class = Reader)]
impl WasmReader {
    /// Creates a reader; omitted limit defaults to 16 MiB of payload bytes.
    ///
    /// # Errors
    ///
    /// Throws if a supplied limit is not an integer in `0..=u32::MAX`.
    /// `null` and `undefined` select the default limit.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] max_payload_len: JsValue,
    ) -> Result<Self, JsError> {
        let limits = ReadLimits {
            max_payload_len: optional_integer(&max_payload_len, "max_payload_len", 0, u32::MAX)?
                .unwrap_or(ReadLimits::default().max_payload_len),
        };
        Ok(Self { reader: Reader::new(limits), max_payload_len: limits.max_payload_len })
    }

    /// Decodes a complete file, optionally requiring a particular schema ID.
    ///
    /// Checks the input length before copying the `Uint8Array` into WASM memory,
    /// then delegates validation and schema decoding to the Rust reader.
    ///
    /// # Errors
    ///
    /// Throws a JavaScript `Error` on invalid input, an unexpected schema, or
    /// failure to construct the decoded JavaScript value. A supplied schema ID
    /// must be an integer in `1..=u16::MAX`; `null` and `undefined` omit the check.
    #[wasm_bindgen(skip_typescript)]
    pub fn decode(
        &self,
        bytes: &js_sys::Uint8Array,
        #[wasm_bindgen(unchecked_optional_param_type = "number | null")] expected_schema: JsValue,
    ) -> Result<DecodedValue, JsError> {
        let expected_schema =
            optional_integer(&expected_schema, "expected_schema", 1, u32::from(u16::MAX))?
                .map(u16::try_from)
                .transpose()?;
        let mut input = self.copy_input(bytes)?;
        let decoded = self.reader.read_envelope(&mut input)?;
        if let Some(expected) = expected_schema
            && expected != decoded.header.schema_id
        {
            return Err(Error::SchemaMismatch { expected, actual: decoded.header.schema_id }.into());
        }
        let value = match decoded.header.schema_id {
            schema::BYTES => js_sys::Uint8Array::from(decoded.payload).into(),
            schema::TEXT => {
                let text = std::str::from_utf8(decoded.payload).map_err(Error::from)?;
                JsValue::from_str(text)
            }
            schema::JSON => {
                let json = serde_json::from_slice(decoded.payload).map_err(Error::from)?;
                json_to_js(json)?
            }
            id => {
                let value = crate::schemas::territory::decode(id, decoded.payload)?;
                let serializer =
                    serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
                use serde::Serialize;
                match value {
                    crate::Value::TerritoryMesh(value) => value.serialize(&serializer)?,
                    crate::Value::TerritoryChunk(value) => value.serialize(&serializer)?,
                    crate::Value::TerritoryProvince(value) => value.serialize(&serializer)?,
                    _ => return Err(Error::UnknownSchema(id).into()),
                }
            }
        };
        Ok(DecodedValue { schema_id: decoded.header.schema_id, value })
    }

    /// Validates and unmasks a file without interpreting its payload schema.
    ///
    /// The result's `value` is a JavaScript-owned `Uint8Array`. Callers decode
    /// its fields according to `schemaId` and release the wrapper with `free()`.
    ///
    /// # Errors
    ///
    /// Throws on invalid framing, checksum failure, an oversized payload, or
    /// a schema mismatch. The required schema must be an integer in `1..=65535`.
    #[wasm_bindgen(js_name = readEnvelope)]
    pub fn read_envelope(
        &self,
        bytes: &js_sys::Uint8Array,
        #[wasm_bindgen(unchecked_param_type = "number")] expected_schema: JsValue,
    ) -> Result<DecodedValue, JsError> {
        let expected =
            optional_integer(&expected_schema, "expected_schema", 1, u32::from(u16::MAX))?
                .ok_or_else(|| JsError::new("expected_schema is required"))?;
        let expected = u16::try_from(expected)?;
        let mut input = self.copy_input(bytes)?;
        let envelope = self.reader.read_envelope(&mut input)?;
        if envelope.header.schema_id != expected {
            return Err(
                Error::SchemaMismatch { expected, actual: envelope.header.schema_id }.into()
            );
        }
        Ok(DecodedValue {
            schema_id: envelope.header.schema_id,
            value: js_sys::Uint8Array::from(envelope.payload).into(),
        })
    }
}

impl WasmReader {
    fn copy_input(&self, bytes: &js_sys::Uint8Array) -> Result<Vec<u8>, JsError> {
        if u64::from(bytes.length()) > u64::from(self.max_payload_len) + HEADER_LEN as u64 {
            return Err(Error::PayloadTooLarge.into());
        }
        Ok(bytes.to_vec())
    }
}

fn optional_integer(
    value: &JsValue,
    name: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, JsError> {
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    // Inspect the JS value before conversion: wasm-bindgen's integer arguments
    // otherwise wrap negatives and overflow, and truncate fractional numbers.
    let number = value
        .as_f64()
        .and_then(|number| bounded_integer(number, min, max))
        .ok_or_else(|| JsError::new(&format!("{name} must be an integer in {min}..={max}")))?;
    Ok(Some(number))
}

fn bounded_integer(number: f64, min: u32, max: u32) -> Option<u32> {
    if !number.is_finite()
        || number.fract() != 0.0
        || number < f64::from(min)
        || number > f64::from(max)
    {
        return None;
    }
    #[expect(clippy::cast_sign_loss, reason = "validated within the unsigned range above")]
    let number = number as u32;
    Some(number)
}

fn json_to_js(value: serde_json::Value) -> Result<JsValue, JsError> {
    use serde_json::Value;

    Ok(match value {
        Value::Null => JsValue::NULL,
        Value::Bool(value) => JsValue::from_bool(value),
        Value::String(value) => JsValue::from_str(&value),
        Value::Number(value) => {
            // Converting i64/u64 through Number would round integers above 2^53.
            if let Some(value) = value.as_i64() {
                js_sys::BigInt::from(value).into()
            } else if let Some(value) = value.as_u64() {
                js_sys::BigInt::from(value).into()
            } else {
                JsValue::from_f64(
                    value.as_f64().ok_or_else(|| JsError::new("invalid JSON number"))?,
                )
            }
        }
        Value::Array(values) => {
            values.into_iter().map(json_to_js).collect::<Result<js_sys::Array, _>>()?.into()
        }
        Value::Object(values) => {
            let entries = js_sys::Array::new();
            for (key, value) in values {
                let entry = js_sys::Array::new();
                entry.push(&JsValue::from_str(&key));
                entry.push(&json_to_js(value)?);
                entries.push(&entry);
            }
            // fromEntries creates own data properties, including "__proto__";
            // assignment-based serializers can lose that key and change prototypes.
            js_sys::Object::from_entries(&entries)
                .map_err(|error| {
                    JsError::new(&format!("could not construct JSON object: {error:?}"))
                })?
                .into()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::bounded_integer;

    #[test]
    fn rejects_invalid_payload_limits() {
        for number in [-1.0, 1.5, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 4_294_967_296.0] {
            assert_eq!(bounded_integer(number, 0, u32::MAX), None);
        }
    }

    #[test]
    fn rejects_invalid_schema_ids() {
        for number in [0.0, -1.0, 1.5, f64::NAN, f64::INFINITY, 65_536.0, 65_537.0] {
            assert_eq!(bounded_integer(number, 1, u32::from(u16::MAX)), None);
        }
    }

    #[test]
    fn accepts_integer_boundaries() {
        for number in [0, 1, u32::MAX] {
            assert_eq!(bounded_integer(f64::from(number), 0, u32::MAX), Some(number));
        }
        for number in [1, u32::from(u16::MAX)] {
            assert_eq!(bounded_integer(f64::from(number), 1, u32::from(u16::MAX)), Some(number));
        }
    }
}
