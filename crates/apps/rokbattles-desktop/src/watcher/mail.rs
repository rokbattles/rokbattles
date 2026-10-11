use std::path::Path;

/// Parses the suffix of `Persistent.Mail.<id>` as a `u128`.
///
/// Only ASCII digits are accepted. Use [`mail_id_for_path`] for ingress identity,
/// where leading zeroes must be preserved.
pub(crate) fn parse_rok_mail_id(filename: &str) -> Option<u128> {
    let rest = filename.strip_prefix("Persistent.Mail.")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u128>().ok()
}

/// Returns the validated mail ID from a path without changing its spelling.
///
/// For example, `Persistent.Mail.001` returns `"001"`, which ingress treats as
/// distinct from `"1"`.
pub(crate) fn mail_id_for_path(path: &str) -> Option<&str> {
    let name = Path::new(path).file_name()?.to_str()?;
    parse_rok_mail_id(name)?;

    name.strip_prefix("Persistent.Mail.")
}

/// Returns a non-empty UTF-8 filename for an upload.
///
/// This does not validate the mail ID; use [`mail_id_for_path`] for that check.
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
