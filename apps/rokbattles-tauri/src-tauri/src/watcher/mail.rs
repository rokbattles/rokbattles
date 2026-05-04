use std::path::Path;

/// Parse the numeric mail id from a RoK mail filename.
pub(crate) fn parse_rok_mail_id(filename: &str) -> Option<u128> {
    let rest = filename.strip_prefix("Persistent.Mail.")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u128>().ok()
}

/// Heuristic header validation to quickly skip non-mail buffers.
pub(crate) fn has_rok_mail_header(buf: &[u8]) -> bool {
    if buf.len() < 32 {
        return false;
    }
    if buf[0] != 0xFF {
        return false;
    }
    if buf[9] != 0x05 || buf[10] != 0x04 {
        return false;
    }
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
    let start = 15;
    let end = start + 9;
    let Some(bytes) = buf.get(start..end) else {
        return false;
    };
    bytes == b"mailScene"
}

/// Extract a non-empty file name for API uploads.
pub(crate) fn file_name_for_upload(path: &Path) -> Option<String> {
    path.file_name().and_then(|s| s.to_str()).filter(|name| !name.is_empty()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rok_mail_id_requires_numeric_suffix() {
        assert_eq!(parse_rok_mail_id("Persistent.Mail.123"), Some(123));
        assert_eq!(parse_rok_mail_id("Persistent.Mail.001"), Some(1));
        assert_eq!(parse_rok_mail_id("Persistent.Mail."), None);
        assert_eq!(parse_rok_mail_id("Persistent.Mail.123a"), None);
        assert_eq!(parse_rok_mail_id("Other.Mail.123"), None);
    }

    #[test]
    fn file_name_for_upload_rejects_missing_names() {
        assert_eq!(
            file_name_for_upload(Path::new("Persistent.Mail.123")),
            Some("Persistent.Mail.123".to_string())
        );
        assert_eq!(file_name_for_upload(Path::new("")), None);
    }

    #[test]
    fn has_rok_mail_header_matches_expected_bytes() {
        let mut buf = vec![0u8; 32];
        buf[0] = 0xFF;
        buf[9] = 0x05;
        buf[10] = 0x04;
        buf[11..15].copy_from_slice(9u32.to_le_bytes().as_slice());
        buf[15..24].copy_from_slice(b"mailScene");

        assert!(has_rok_mail_header(&buf));

        let mut wrong = buf.clone();
        wrong[0] = 0x00;
        assert!(!has_rok_mail_header(&wrong));
    }
}
