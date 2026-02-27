use std::collections::HashMap;

use serde_json::Value;

pub(crate) fn parse_positive_governor_id_str(value: &str) -> Option<i64> {
    let parsed = value.trim().parse::<i64>().ok()?;
    (parsed > 0).then_some(parsed)
}

pub(crate) fn parse_positive_governor_id_from_query(
    params: &HashMap<String, String>,
) -> Option<i64> {
    params
        .get("governorId")
        .and_then(|value| parse_positive_governor_id_str(value))
}

pub(crate) fn parse_positive_governor_id_from_json(value: Option<&Value>) -> Option<i64> {
    let value = value?;

    let parsed = match value {
        Value::Number(number) => {
            if let Some(parsed) = number.as_i64() {
                Some(parsed)
            } else if let Some(parsed) = number.as_u64() {
                i64::try_from(parsed).ok()
            } else if let Some(parsed) = number.as_f64() {
                if parsed.is_finite()
                    && parsed.fract() == 0.0
                    && parsed >= i64::MIN as f64
                    && parsed <= i64::MAX as f64
                {
                    Some(parsed as i64)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Value::String(value) => parse_positive_governor_id_str(value),
        _ => None,
    };

    parsed.filter(|governor_id| *governor_id > 0)
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

    #[test]
    fn parses_governor_id_from_json_number_and_string() {
        assert_eq!(
            parse_positive_governor_id_from_json(Some(&Value::Number(123.into()))),
            Some(123)
        );
        assert_eq!(
            parse_positive_governor_id_from_json(Some(&Value::String("456".to_string()))),
            Some(456)
        );
    }

    #[test]
    fn parses_governor_id_from_query() {
        let params = HashMap::from([("governorId".to_string(), "789".to_string())]);
        assert_eq!(parse_positive_governor_id_from_query(&params), Some(789));
    }
}
