use crate::error::ApiError;

pub(crate) fn parse_optional_i64(
    value: Option<&str>,
    error_message: &'static str,
) -> Result<Option<i64>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };

    value.parse::<i64>().map(Some).map_err(|_error| ApiError::bad_request(error_message))
}

#[cfg(test)]
mod tests {
    use super::parse_optional_i64;

    #[test]
    fn parses_i64_and_none() {
        assert_eq!(parse_optional_i64(Some("42"), "bad value").expect("should parse"), Some(42));
        assert_eq!(parse_optional_i64(None, "bad value").expect("should parse"), None);
    }

    #[test]
    fn rejects_invalid_numbers() {
        parse_optional_i64(Some("abc"), "bad value").unwrap_err();
    }
}
