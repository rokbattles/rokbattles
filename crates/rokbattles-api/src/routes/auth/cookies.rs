use crate::auth::SESSION_COOKIE_NAME;

pub(crate) const ACTIVE_GOVERNOR_COOKIE_NAME: &str = "_rokb_active_governor";
const SESSION_COOKIE_FLAGS: &str = "Path=/; HttpOnly; Secure; SameSite=Lax";
const ACTIVE_GOVERNOR_COOKIE_FLAGS: &str = "Path=/; HttpOnly; Secure; SameSite=Lax";

pub(super) fn build_set_session_cookie(session_id: &str, max_age_seconds: u64) -> String {
    format!("{SESSION_COOKIE_NAME}={session_id}; Max-Age={max_age_seconds}; {SESSION_COOKIE_FLAGS}")
}

pub(super) fn build_clear_session_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Max-Age=0; {SESSION_COOKIE_FLAGS}")
}

pub(super) fn build_set_active_governor_cookie(governor_id: i64, max_age_seconds: u64) -> String {
    format!(
        "{ACTIVE_GOVERNOR_COOKIE_NAME}={governor_id}; Max-Age={max_age_seconds}; {ACTIVE_GOVERNOR_COOKIE_FLAGS}"
    )
}

pub(super) fn build_clear_active_governor_cookie() -> String {
    format!("{ACTIVE_GOVERNOR_COOKIE_NAME}=; Max-Age=0; {ACTIVE_GOVERNOR_COOKIE_FLAGS}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_set_cookie_header_value() {
        let cookie = build_set_session_cookie("abc123", 60);
        assert_eq!(
            cookie,
            "_rokb_session=abc123; Max-Age=60; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }

    #[test]
    fn builds_clear_cookie_header_value() {
        let cookie = build_clear_session_cookie();
        assert_eq!(
            cookie,
            "_rokb_session=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }

    #[test]
    fn builds_set_active_governor_cookie_header_value() {
        let cookie = build_set_active_governor_cookie(12345, 60);
        assert_eq!(
            cookie,
            "_rokb_active_governor=12345; Max-Age=60; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }

    #[test]
    fn builds_clear_active_governor_cookie_header_value() {
        let cookie = build_clear_active_governor_cookie();
        assert_eq!(
            cookie,
            "_rokb_active_governor=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax"
        );
    }
}
