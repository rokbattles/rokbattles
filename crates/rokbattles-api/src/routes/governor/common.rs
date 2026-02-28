pub(crate) fn parse_positive_governor_id_str(value: &str) -> Option<i64> {
    let parsed = value.trim().parse::<i64>().ok()?;
    (parsed > 0).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_positive_governor_id_from_string() {
        assert_eq!(parse_positive_governor_id_str("123"), Some(123));
        assert_eq!(parse_positive_governor_id_str(" 456 "), Some(456));
    }

    #[test]
    fn rejects_non_positive_or_invalid_governor_ids() {
        assert_eq!(parse_positive_governor_id_str("0"), None);
        assert_eq!(parse_positive_governor_id_str("-1"), None);
        assert_eq!(parse_positive_governor_id_str("abc"), None);
    }
}
