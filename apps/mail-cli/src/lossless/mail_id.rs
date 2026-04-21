use mail_decoder::{LosslessContainer, LosslessValue};

pub(super) fn extract_lossless_mail_id(value: &LosslessValue) -> Option<String> {
    match value {
        LosslessValue::Container(LosslessContainer::Object(object)) => {
            for entry in &object.entries {
                if (entry.key.eq_ignore_ascii_case("id")
                    || entry.key.eq_ignore_ascii_case("mail_id"))
                    && let Some(id) = lossless_value_to_string(&entry.value)
                {
                    return Some(id);
                }
            }
            for entry in &object.entries {
                if let Some(id) = extract_lossless_mail_id(&entry.value) {
                    return Some(id);
                }
            }
            None
        }
        LosslessValue::Container(LosslessContainer::Array(array)) => {
            array.items.iter().find_map(extract_lossless_mail_id)
        }
        _ => None,
    }
}

pub(super) fn validate_mail_id(mail_id: &str) -> Result<(), String> {
    if mail_id.is_empty() {
        return Err("mail id cannot be empty".to_string());
    }
    if mail_id.contains(['/', '\\']) {
        return Err("mail id cannot contain path separators".to_string());
    }
    Ok(())
}

fn lossless_value_to_string(value: &LosslessValue) -> Option<String> {
    match value {
        LosslessValue::String { value } => Some(value.clone()),
        LosslessValue::F64 { raw } => {
            let num = f64::from_be_bytes(*raw);
            if !num.is_finite() {
                return None;
            }
            let rounded = num.round();
            if (num - rounded).abs() > f64::EPSILON {
                return None;
            }
            if !(0.0..=9_007_199_254_740_992.0).contains(&rounded) {
                return None;
            }
            Some(format!("{rounded:.0}"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mail_decoder::{LosslessContainer, LosslessEntry, LosslessObject, LosslessValue};

    use super::*;

    #[test]
    fn validate_mail_id_rejects_empty_mail_id() {
        assert_eq!(validate_mail_id("").unwrap_err(), "mail id cannot be empty");
    }

    #[test]
    fn validate_mail_id_rejects_path_separators() {
        assert_eq!(
            validate_mail_id("../../outside").unwrap_err(),
            "mail id cannot contain path separators"
        );
    }

    #[test]
    fn extract_lossless_mail_id_accepts_whole_numeric_mail_id_from_f64() {
        let value = LosslessValue::Container(LosslessContainer::Object(LosslessObject {
            entries: vec![LosslessEntry {
                key: "id".to_string(),
                value: LosslessValue::F64 { raw: 123.0f64.to_be_bytes() },
            }],
            terminator: None,
        }));

        assert_eq!(extract_lossless_mail_id(&value).as_deref(), Some("123"));
    }
}
