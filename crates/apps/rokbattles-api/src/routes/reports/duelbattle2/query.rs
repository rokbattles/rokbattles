use std::collections::HashMap;

use crate::{error::ApiError, routes::reports::common::query::parse_optional_i64};

#[derive(Debug, Clone, Copy)]
pub(crate) struct DuelBattle2Request {
    pub before_cursor: Option<i64>,
    pub after_cursor: Option<i64>,
}

impl DuelBattle2Request {
    pub fn sort_direction(&self) -> i32 {
        if self.before_cursor.is_some() { 1 } else { -1 }
    }
}

pub(crate) fn parse_duelbattle2_request(
    params: &HashMap<String, String>,
) -> Result<DuelBattle2Request, ApiError> {
    let before_cursor =
        parse_optional_i64(params.get("before").map(String::as_str), "Invalid before cursor")?;
    let after_cursor = if before_cursor.is_some() {
        None
    } else {
        parse_optional_i64(params.get("after").map(String::as_str), "Invalid after cursor")?
    };

    Ok(DuelBattle2Request { before_cursor, after_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_numeric_after_cursor() {
        let result =
            parse_duelbattle2_request(&HashMap::from([("after".to_string(), "bad".to_string())]));
        result.unwrap_err();
    }

    #[test]
    fn prefers_before_when_both_are_present() {
        let parsed = parse_duelbattle2_request(&HashMap::from([
            ("before".to_string(), "100".to_string()),
            ("after".to_string(), "50".to_string()),
        ]))
        .expect("request should parse");

        assert_eq!(parsed.before_cursor, Some(100));
        assert_eq!(parsed.after_cursor, None);
    }
}
