use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Map, Number, Value};

#[derive(Clone, Copy)]
struct Cursor<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, offset: 0 }
    }

    fn eof(&self) -> bool {
        self.offset >= self.buffer.len()
    }

    fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.offset)
    }

    fn u8(&mut self) -> Result<u8> {
        if self.remaining() < 1 {
            return Err(anyhow!("u8 past EOF"));
        }
        let val = self.buffer[self.offset];
        self.offset += 1;
        Ok(val)
    }

    fn u32_le(&mut self) -> Result<u32> {
        if self.remaining() < 4 {
            return Err(anyhow!("u32_le past EOF"));
        }
        let val = u32::from_le_bytes(self.buffer[self.offset..self.offset + 4].try_into()?);
        self.offset += 4;
        Ok(val)
    }

    fn f32_le(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32_le()?))
    }

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

fn json_normalize_float(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f >= (i64::MIN as f64) && f <= (i64::MAX as f64) {
        Value::Number(Number::from(f as i64))
    } else {
        Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

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

#[derive(Debug, Serialize)]
pub struct Mail {
    pub sections: Vec<Value>,
}

pub fn decode(buffer: &[u8]) -> Result<Mail> {
    let sections = parse_sections(buffer)?;

    Ok(Mail { sections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    fn has_mailscene_header(buf: &[u8]) -> bool {
        // Require a small minimum size so all fixed offsets used below are safe.
        if buf.len() < 32 {
            return false;
        }

        // Leading sentinel byte for this record type.
        if buf[0] != 0xFF {
            return false;
        }

        // 0x05 denotes a nested object; 0x04 denotes a UTF‑8 string key tag.
        if buf[9] != 0x05 || buf[10] != 0x04 {
            return false;
        }

        // Read the length (LE u32) of the upcoming key; expect "mailScene" (9 bytes).
        let len = {
            let start = 11;
            let end = start + 4;
            let Some(bytes) = buf.get(start..end) else {
                return false;
            };
            u32::from_le_bytes(bytes.try_into().unwrap())
        };

        if len != 9 {
            return false;
        }

        // Verify the key bytes equal "mailScene".
        let start = 15;
        let end = start + 9;
        let Some(bytes) = buf.get(start..end) else {
            return false;
        };
        bytes == b"mailScene"
    }

    #[test]
    fn validate_mailscene_header_bytes() {
        // Build a small synthetic buffer with sufficient capacity for all offsets we touch.
        let mut buf = vec![0u8; 32];

        // Fixed header bytes per has_mailscene_header contract.
        buf[0] = 0xFF; // leading sentinel
        buf[9] = 0x05; // begin nested object
        buf[10] = 0x04; // string key tag

        // u32 LE length for "mailScene" (9 bytes).
        let len_bytes = 9u32.to_le_bytes();
        buf[11..15].copy_from_slice(&len_bytes);

        // ASCII key bytes: "mailScene".
        buf[15..24].copy_from_slice(b"mailScene");

        // The exact header should be recognized.
        assert!(has_mailscene_header(&buf), "expected valid header to pass");

        // Truncated input should be safely rejected (no panics, returns false).
        assert!(!has_mailscene_header(&buf[..16]), "short buffers must fail");

        // Flip the leading sentinel to ensure we detect the mismatch.
        let mut wrong0 = buf.clone();
        wrong0[0] = 0x00;
        assert!(
            !has_mailscene_header(&wrong0),
            "wrong leading byte must fail"
        );

        // Zero the nested-object tag to test the tag check.
        let mut wrong_tags = buf.clone();
        wrong_tags[9] = 0x00;
        assert!(
            !has_mailscene_header(&wrong_tags),
            "wrong tag at index 9 must fail"
        );

        // Zero the key tag to test the adjacent tag check.
        let mut wrong_tags2 = buf.clone();
        wrong_tags2[10] = 0x00;
        assert!(
            !has_mailscene_header(&wrong_tags2),
            "wrong tag at index 10 must fail"
        );

        // Write an incorrect LE length; should reject even if the bytes spell "mailScene".
        let mut wrong_len = buf.clone();
        wrong_len[11..15].copy_from_slice(8u32.to_le_bytes().as_slice());
        assert!(
            !has_mailscene_header(&wrong_len),
            "wrong LE length must fail"
        );

        // Corrupt the key bytes; should be rejected even with correct length.
        let mut wrong_scene = buf.clone();
        wrong_scene[15..24].copy_from_slice(b"mailScena");
        assert!(
            !has_mailscene_header(&wrong_scene),
            "wrong scene string must fail"
        );
    }

    #[test]
    fn validate_sample_mail_has_header() -> Result<()> {
        // Resolve sample path relative to the crate's manifest directory to keep CI stable.
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let samples_dir = manifest_dir.join("../../samples");
        let bin_path = samples_dir.join("Persistent.Mail.100439187175234501131");

        // Read the raw binary; surface any IO issues with helpful context.
        let input = fs::read(&bin_path)
            .map_err(|e| anyhow::anyhow!("failed to read sample {:?}: {e}", bin_path))?;

        // The header check is a heuristic; passing here indicates the sample looks like
        // a valid RoK mail payload at a glance.
        assert!(
            has_mailscene_header(&input),
            "expected sample mail to have the mailScene header"
        );

        Ok(())
    }
}
