use std::path::Path;

/// Parse the numeric mail id from a RoK mail filename.
pub(crate) fn parse_rok_mail_id(filename: &str) -> Option<u128> {
    let rest = filename.strip_prefix("Persistent.Mail.")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u128>().ok()
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
}
