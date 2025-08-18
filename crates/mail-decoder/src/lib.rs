//! Rise of Kingdoms (RoK) mail decoder: converts the game's tagged, binary mail
//! payloads into JSON-like objects (serde_json::Value).
//!
//! This decoder targets RoK mail/report formats specifically and is not intended
//! as a general-purpose binary-to-JSON tool. It tolerates unknown tags and minor
//! format drift to remain resilient across versions.
//!
//! The stream is composed of objects and key/value pairs encoded with simple tags:
//! - 0x04: start of a UTF-8 key (little-endian u32 length, followed by bytes)
//! - 0x01: boolean value (u8 0/1)
//! - 0x02: f32 little-endian (normalized to JSON number, possibly integer)
//! - 0x03: f64 big-endian (normalized to JSON number)
//! - 0x04: string value (little-endian u32 length + bytes)
//! - 0x05: nested object; terminated by a 0xff sentinel
//! - 0xff: end-of-object sentinel
//!
//! Top-level data is read as a sequence of "sections" (objects). We search forward
//! for plausible keys to recover if the stream has padding or unrelated noise.

use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Map, Number, Value};

/// A lightweight, bounds-checked reader over a byte slice.
///
/// Cursor provides convenience methods to read typed values while keeping track
/// of the current offset. Every read advances the offset and performs EOF checks,
/// returning an error if insufficient bytes remain.
#[derive(Clone, Copy)]
struct Cursor<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    /// Create a new cursor starting at offset 0.
    fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Returns true when the cursor has reached or passed the end of the buffer.
    fn eof(&self) -> bool {
        self.offset >= self.buffer.len()
    }

    /// Number of bytes remaining to be read from the current offset.
    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.offset)
    }

    /// Read a single byte.
    ///
    /// Errors if there are no bytes left.
    fn u8(&mut self) -> Result<u8> {
        if self.remaining() < 1 {
            return Err(anyhow!("u8 past EOF"));
        }
        let val = self.buffer[self.offset];
        self.offset += 1;
        Ok(val)
    }

    /// Read a little-endian u32.
    ///
    /// Errors if fewer than 4 bytes remain.
    fn u32_le(&mut self) -> Result<u32> {
        if self.remaining() < 4 {
            return Err(anyhow!("u32_le past EOF"));
        }
        let val = u32::from_le_bytes(self.buffer[self.offset..self.offset + 4].try_into()?);
        self.offset += 4;
        Ok(val)
    }

    /// Read a little-endian f32 by reusing u32_le and converting from bits.
    fn f32_le(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32_le()?))
    }

    /// Read a big-endian f64.
    ///
    /// Errors if fewer than 8 bytes remain.
    fn f64_be(&mut self) -> Result<f64> {
        if self.remaining() < 8 {
            return Err(anyhow!("f64_be past EOF"));
        }
        let val = f64::from_bits(u64::from_be_bytes(
            self.buffer[self.offset..self.offset + 8].try_into()?,
        ));
        self.offset += 8;
        Ok(val)
    }

    /// Read a UTF-8 string with a little-endian u32 length prefix.
    ///
    /// Errors if the length extends past EOF or if the bytes are not valid UTF-8.
    fn str_utf8(&mut self) -> Result<String> {
        let len = self.u32_le()? as usize;
        if self.remaining() < len {
            return Err(anyhow!("str_utf8 past EOF"));
        }
        let str = std::str::from_utf8(&self.buffer[self.offset..self.offset + len])?.to_owned();
        self.offset += len;
        Ok(str)
    }
}

/// Parse a value given a "head" tag byte.
///
/// Recognized tags:
/// - 0x01: bool
/// - 0x02: f32 (LE)
/// - 0x03: f64 (BE)
/// - 0x04: UTF-8 string
/// - 0x05: nested object (terminated by 0xff)
///
/// Unknown tags yield `Value::Null`.
///
/// The `key_path` is used only to annotate context for nested objects, aiding
/// diagnostics and future error messages.
fn parse_value(head: u8, cursor: &mut Cursor, key_path: &str) -> Result<Value> {
    match head {
        0x01 => Ok(Value::Bool(cursor.u8()? != 0)),
        0x02 => {
            let f = cursor.f32_le()? as f64;
            Ok(json_normalize_float(f))
        }
        0x03 => {
            let f = cursor.f64_be()?;
            Ok(json_normalize_float(f))
        }
        0x04 => Ok(Value::String(cursor.str_utf8()?)),
        0x05 => parse_object(cursor, Some(key_path)),
        // For forward compatibility, treat unknown/unsupported tags as null
        _ => Ok(Value::Null),
    }
}

/// Create a serde_json `Value::Number` from a floating-point value.
///
/// - If the float is finite and represents an integer within the i64 range,
///   encode it as an integer JSON number to avoid trailing `.0` in output.
/// - Otherwise, encode as an f64 JSON number if finite.
/// - Non-finite values (NaN/Inf) are converted to `Value::Null` since JSON
///   does not support them.
fn json_normalize_float(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f >= (i64::MIN as f64) && f <= (i64::MAX as f64) {
        Value::Number(Number::from(f as i64))
    } else {
        Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// Parse an object (a map of string keys to values) from the current cursor position.
///
/// The format is a sequence of:
/// - key tag (0x04)
/// - u32 LE length
/// - UTF-8 key bytes
/// - head tag (value type)
/// - value bytes depending on the head tag
///
/// until an 0xff sentinel is encountered.
///
/// Unknown tags are skipped to maintain forward compatibility; an unexpected EOF
/// after a key inserts that key with `null` value to preserve shape.
fn parse_object(cursor: &mut Cursor, parent: Option<&str>) -> Result<Value> {
    let mut obj = Map::new();
    while !cursor.eof() {
        let tag = cursor.u8()?;
        if tag == 0xff {
            // 0xff marks the end of the current object.
            return Ok(Value::Object(obj));
        }
        if tag != 0x04 {
            // Skip unknown/non-key tags; they may be padding or newer fields we don't understand.
            continue;
        }
        let key = cursor.str_utf8()?;
        if cursor.eof() {
            // If a key is present but no head/value follows, record it as null and stop.
            obj.insert(key, Value::Null);
            return Ok(Value::Object(obj));
        }
        let head = cursor.u8()?;
        // Build a dotted path for diagnostics (e.g., parent.childKey).
        let key_path = if let Some(p) = parent {
            format!("{p}.{key}")
        } else {
            key.clone()
        };
        let val = parse_value(head, cursor, &key_path)?;
        obj.insert(key, val);
    }
    // If EOF occurs without an explicit 0xff, return what we have.
    Ok(Value::Object(obj))
}

/// Advance the cursor to the next plausible key tag (0x04).
///
/// This function implements a heuristic "resync" to find the start of a key:
/// - Look for 0x04
/// - Peek a little-endian u32 length and require it to be within a sane range (1..1024)
/// - Ensure the following bytes are printable ASCII (reduces false positives)
///
/// If all checks pass, the cursor's offset is set to the candidate key start and
/// the function returns true. Otherwise, it keeps scanning forward.
///
/// Returns false when no suitable candidate is found or the buffer tail is too short.
fn next_key(cursor: &mut Cursor) -> bool {
    let buf_len = cursor.buffer.len();
    let mut off = cursor.offset;

    // Require at least: 1 tag byte + 4 length bytes + >=2 key chars for a minimally plausible key.
    while off + 7 <= buf_len {
        if cursor.buffer[off] != 0x04 {
            off += 1;
            continue;
        }

        // Not enough bytes to read the length -> no more valid keys possible.
        if off + 5 > buf_len {
            return false;
        }

        let len_bytes = &cursor.buffer[off + 1..off + 5];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

        // Filter out absurd lengths; this prevents scanning across large stretches mistakenly.
        if !(1..1024).contains(&len) {
            off += 1;
            continue;
        }

        let start = off + 5;
        let Some(end) = start.checked_add(len).filter(|&e| e <= buf_len) else {
            off += 1;
            continue;
        };

        if len >= 2 {
            let slice = &cursor.buffer[start..end];
            // Heuristic: keys are printable ASCII; this dramatically lowers false positives.
            if slice.is_ascii() && slice.iter().all(|&b| (b' '..=b'~').contains(&b)) {
                cursor.offset = off;
                return true;
            }
        }

        off += 1
    }
    false
}

/// Parse the input buffer into a list of top-level sections.
///
/// Each section corresponds to an object parsed starting at the next plausible key.
/// The loop stops when:
/// - No next key can be found,
/// - The parsed object is empty and the cursor cannot advance (infinite-loop guard),
/// - Or EOF is reached.
fn parse_sections(buffer: &[u8]) -> Result<Vec<Value>> {
    let mut cursor = Cursor::new(buffer);
    let mut sections = Vec::new();

    while !cursor.eof() {
        if !next_key(&mut cursor) {
            break;
        }

        // Track progress to avoid infinite loops on malformed inputs.
        let before = cursor.offset;
        let obj = parse_object(&mut cursor, None)?;
        // Only keep non-empty objects to reduce noise.
        if obj.as_object().map(|m| !m.is_empty()).unwrap_or(false) {
            sections.push(obj)
        }
        if cursor.offset <= before {
            // If we didn't advance, abort to avoid getting stuck.
            break;
        }
    }
    Ok(sections)
}

/// Top-level decoded mail representation.
///
/// This is designed to be serialized as JSON, with `sections` holding an ordered
/// list of objects recovered from the binary payload.
#[derive(Debug, Serialize)]
pub struct Mail {
    pub sections: Vec<Value>,
}

/// Decode a binary buffer into a `Mail` structure.
///
/// This function is infallible with respect to unknown tags (they are skipped or set to null)
/// but can fail on hard I/O-like conditions (e.g., truncated lengths or invalid UTF-8) where
/// recovery would be ambiguous.
pub fn decode(buffer: &[u8]) -> Result<Mail> {
    let sections = parse_sections(buffer)?;

    Ok(Mail { sections })
}
