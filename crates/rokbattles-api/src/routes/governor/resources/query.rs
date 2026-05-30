use std::collections::HashMap;

use crate::{
    error::ApiError,
    routes::governor::{common::parse_default_governor_date_range, date_range::GovernorDateRange},
};

#[derive(Debug, Clone)]
pub(crate) struct ResourcesRequest {
    pub range: GovernorDateRange,
}

pub(crate) fn parse_resources_request(
    params: &HashMap<String, String>,
) -> Result<ResourcesRequest, ApiError> {
    let range = parse_default_governor_date_range(params)?;
    Ok(ResourcesRequest { range })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resources_request_resolves_date_range() {
        let request = parse_resources_request(&HashMap::from([
            ("start".to_string(), "2025-02-03".to_string()),
            ("end".to_string(), "2025-02-04".to_string()),
        ]))
        .expect("request");
        assert_eq!(request.range.start, "2025-02-03");
        assert_eq!(request.range.end, "2025-02-04");
    }
}
