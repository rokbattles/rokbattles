use crate::error::RokGtError;
use crate::models::{KingdomListRequest, KingdomMemberRequest};
use serde::Serialize;

pub(super) fn validate_kingdom_list_request(
    request: &KingdomListRequest,
) -> Result<(), RokGtError> {
    if request.page == 0 {
        return Err(RokGtError::InvalidRequest {
            field: "page",
            reason: "must be >= 1",
        });
    }
    if request.size == 0 || request.size > 20 {
        return Err(RokGtError::InvalidRequest {
            field: "size",
            reason: "must be between 1 and 20",
        });
    }
    if let Some(server_id) = &request.server_id
        && server_id.trim().is_empty()
    {
        return Err(RokGtError::InvalidRequest {
            field: "server_id",
            reason: "must not be blank when provided",
        });
    }
    Ok(())
}

pub(super) fn validate_server_id(server_id: &str) -> Result<&str, RokGtError> {
    let trimmed = server_id.trim();
    if trimmed.is_empty() {
        return Err(RokGtError::InvalidRequest {
            field: "server_id",
            reason: "must not be blank",
        });
    }
    Ok(trimmed)
}

#[derive(Serialize)]
pub(super) struct KingdomMemberQuery<'a> {
    start: &'a str,
    end: &'a str,
    search: &'a str,
    server_id: &'a str,
}

pub(super) fn normalize_kingdom_member_request<'a>(
    request: &'a KingdomMemberRequest,
) -> Result<KingdomMemberQuery<'a>, RokGtError> {
    let start = validate_date_yyyy_mm_dd("start", &request.start)?;
    let end = validate_date_yyyy_mm_dd("end", &request.end)?;
    let server_id = validate_server_id(&request.server_id)?;
    let search = request.search.trim();

    Ok(KingdomMemberQuery {
        start,
        end,
        search,
        server_id,
    })
}

fn validate_date_yyyy_mm_dd<'a>(
    field: &'static str,
    value: &'a str,
) -> Result<&'a str, RokGtError> {
    let trimmed = value.trim();
    if trimmed.len() != 10 {
        return Err(RokGtError::InvalidRequest {
            field,
            reason: "must be YYYY-MM-DD",
        });
    }

    let bytes = trimmed.as_bytes();
    let is_valid = bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit);

    if !is_valid {
        return Err(RokGtError::InvalidRequest {
            field,
            reason: "must be YYYY-MM-DD",
        });
    }

    Ok(trimmed)
}

#[cfg(test)]
pub(super) fn validate_kingdom_member_request(
    request: &KingdomMemberRequest,
) -> Result<(), RokGtError> {
    let _ = normalize_kingdom_member_request(request)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_validation_rejects_invalid_pagination() {
        let mut req = KingdomListRequest::default();
        req.page = 0;
        let err = validate_kingdom_list_request(&req).expect_err("page validation should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "page", .. }
        ));
    }

    #[test]
    fn request_validation_rejects_size_above_max() {
        let mut req = KingdomListRequest::default();
        req.size = 21;
        let err = validate_kingdom_list_request(&req).expect_err("size validation should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "size", .. }
        ));
    }

    #[test]
    fn kingdom_information_request_rejects_blank_server_id() {
        let err = validate_server_id("   ").expect_err("server id validation should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest {
                field: "server_id",
                ..
            }
        ));
    }

    #[test]
    fn kingdom_member_request_rejects_blank_date_fields() {
        let err =
            validate_kingdom_member_request(&KingdomMemberRequest::new(" ", "2026-02-17", "2804"))
                .expect_err("start validation should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "start", .. }
        ));

        let err =
            validate_kingdom_member_request(&KingdomMemberRequest::new("2026-02-17", "", "2804"))
                .expect_err("end validation should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "end", .. }
        ));
    }

    #[test]
    fn kingdom_member_request_rejects_invalid_date_format() {
        let err = validate_kingdom_member_request(&KingdomMemberRequest::new(
            "2026/02/17",
            "2026-02-17",
            "2804",
        ))
        .expect_err("start date format should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "start", .. }
        ));

        let err = validate_kingdom_member_request(&KingdomMemberRequest::new(
            "2026-02-17",
            "17-02-2026",
            "2804",
        ))
        .expect_err("end date format should fail");
        assert!(matches!(
            err,
            RokGtError::InvalidRequest { field: "end", .. }
        ));
    }
}
