use anyhow::{Result, anyhow};
use memchr::memchr;
use serde::{Deserialize, Serialize};
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

    fn str_utf8(&mut self) -> Result<&'a str> {
        let len = self.u32_le()? as usize;
        if self.remaining() < len {
            return Err(anyhow!("str_utf8 past EOF"));
        }
        let str = std::str::from_utf8(&self.buffer[self.offset..self.offset + len])?;
        self.offset += len;
        Ok(str)
    }
}

fn normalize_number(f: f64) -> Option<Number> {
    if f.is_finite() && f.fract() == 0.0 && f >= (i64::MIN as f64) && f <= (i64::MAX as f64) {
        Some(Number::from(f as i64))
    } else {
        Number::from_f64(f)
    }
}

#[derive(Debug, Clone)]
pub enum ValueBorrowed<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(&'a str),
    Object(Vec<(&'a str, ValueBorrowed<'a>)>),
}

impl<'a> ValueBorrowed<'a> {
    fn into_owned(self) -> Value {
        match self {
            ValueBorrowed::Null => Value::Null,
            ValueBorrowed::Bool(b) => Value::Bool(b),
            ValueBorrowed::Number(n) => Value::Number(n),
            ValueBorrowed::String(s) => Value::String(s.to_owned()),
            ValueBorrowed::Object(entries) => {
                let mut map = Map::with_capacity(entries.len());
                for (k, v) in entries {
                    map.insert(k.to_owned(), v.into_owned());
                }
                Value::Object(map)
            }
        }
    }
}

fn parse_value<'a>(head: u8, cursor: &mut Cursor<'a>) -> Result<ValueBorrowed<'a>> {
    Ok(match head {
        0x01 => ValueBorrowed::Bool(cursor.u8()? != 0),
        0x02 => normalize_number(cursor.f32_le()? as f64)
            .map(ValueBorrowed::Number)
            .unwrap_or(ValueBorrowed::Null),
        0x03 => normalize_number(cursor.f64_be()?)
            .map(ValueBorrowed::Number)
            .unwrap_or(ValueBorrowed::Null),
        0x04 => ValueBorrowed::String(cursor.str_utf8()?),
        0x05 => parse_object(cursor)?,
        _ => ValueBorrowed::Null,
    })
}

fn parse_object<'a>(cursor: &mut Cursor<'a>) -> Result<ValueBorrowed<'a>> {
    let mut entries: Vec<(&'a str, ValueBorrowed<'a>)> = Vec::new();
    while !cursor.eof() {
        let tag = cursor.u8()?;
        if tag == 0xff {
            return Ok(ValueBorrowed::Object(entries));
        }
        if tag != 0x04 {
            continue;
        }
        let key = cursor.str_utf8()?;
        if cursor.eof() {
            entries.push((key, ValueBorrowed::Null));
            return Ok(ValueBorrowed::Object(entries));
        }
        let head = cursor.u8()?;
        let val = parse_value(head, cursor)?;
        entries.push((key, val));
    }
    Ok(ValueBorrowed::Object(entries))
}

fn next_key(cursor: &mut Cursor) -> bool {
    let buf = cursor.buffer;
    let buf_len = buf.len();
    let mut search = cursor.offset;

    // Require at least: 1 tag byte + 4 length bytes + >=2 key chars for a minimally plausible key.
    while search + 7 <= buf_len {
        let Some(rel) = memchr(0x04, &buf[search..]) else {
            return false;
        };
        let off = search + rel;

        // Not enough bytes to read the length -> no more valid keys possible.
        if off + 5 > buf_len {
            return false;
        }

        let len_bytes = &buf[off + 1..off + 5];
        let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;

        // Filter out absurd lengths; this prevents scanning across large stretches mistakenly.
        if !(1..1024).contains(&len) {
            search = off + 1;
            continue;
        }

        let start = off + 5;
        let Some(end) = start.checked_add(len).filter(|&e| e <= buf_len) else {
            search = off + 1;
            continue;
        };

        if len >= 2 {
            let slice = &buf[start..end];
            if slice.is_ascii() && slice.iter().all(|&b| (b' '..=b'~').contains(&b)) {
                cursor.offset = off;
                return true;
            }
        }

        search = off + 1;
    }
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mail {
    pub sections: Vec<Value>,
}

#[derive(Debug)]
pub struct MailBorrowed<'a> {
    pub sections: Vec<ValueBorrowed<'a>>,
}

impl<'a> MailBorrowed<'a> {
    pub fn into_owned(self) -> Mail {
        Mail {
            sections: self
                .sections
                .into_iter()
                .map(ValueBorrowed::into_owned)
                .collect(),
        }
    }
}

fn parse_sections<'a>(buffer: &'a [u8]) -> Result<Vec<ValueBorrowed<'a>>> {
    let mut cursor = Cursor::new(buffer);
    let mut sections: Vec<ValueBorrowed<'a>> = Vec::new();

    while !cursor.eof() {
        if !next_key(&mut cursor) {
            break;
        }

        let before = cursor.offset;
        let obj = parse_object(&mut cursor)?;
        let keep = match &obj {
            ValueBorrowed::Object(entries) => !entries.is_empty(),
            _ => false,
        };
        if keep {
            sections.push(obj)
        }
        if cursor.offset <= before {
            break;
        }
    }
    Ok(sections)
}

pub fn decode(buffer: &[u8]) -> Result<MailBorrowed<'_>> {
    let sections = parse_sections(buffer)?;
    Ok(MailBorrowed { sections })
}

pub fn has_rok_mail_header(buf: &[u8]) -> bool {
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
        u32::from_le_bytes(bytes.try_into().unwrap_or([0; 4]))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

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
        assert!(has_rok_mail_header(&buf), "expected valid header to pass");

        // Truncated input should be safely rejected (no panics, returns false).
        assert!(!has_rok_mail_header(&buf[..16]), "short buffers must fail");

        // Flip the leading sentinel to ensure we detect the mismatch.
        let mut wrong0 = buf.clone();
        wrong0[0] = 0x00;
        assert!(
            !has_rok_mail_header(&wrong0),
            "wrong leading byte must fail"
        );

        // Zero the nested-object tag to test the tag check.
        let mut wrong_tags = buf.clone();
        wrong_tags[9] = 0x00;
        assert!(
            !has_rok_mail_header(&wrong_tags),
            "wrong tag at index 9 must fail"
        );

        // Zero the key tag to test the adjacent tag check.
        let mut wrong_tags2 = buf.clone();
        wrong_tags2[10] = 0x00;
        assert!(
            !has_rok_mail_header(&wrong_tags2),
            "wrong tag at index 10 must fail"
        );

        // Write an incorrect LE length; should reject even if the bytes spell "mailScene".
        let mut wrong_len = buf.clone();
        wrong_len[11..15].copy_from_slice(8u32.to_le_bytes().as_slice());
        assert!(
            !has_rok_mail_header(&wrong_len),
            "wrong LE length must fail"
        );

        // Corrupt the key bytes; should be rejected even with correct length.
        let mut wrong_scene = buf.clone();
        wrong_scene[15..24].copy_from_slice(b"mailScena");
        assert!(
            !has_rok_mail_header(&wrong_scene),
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
            has_rok_mail_header(&input),
            "expected sample mail to have the mailScene header"
        );

        Ok(())
    }
}
