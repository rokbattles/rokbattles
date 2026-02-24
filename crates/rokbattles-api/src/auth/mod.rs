use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::COOKIE;
use axum::http::request::Parts;
use tracing::warn;

use crate::db::{SessionRecord, UserRecord};
use crate::error::ApiError;
use crate::state::AppState;

/// Authenticated request context built from the `sid` cookie.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub sid: String,
    pub session: SessionRecord,
    pub user: UserRecord,
}

impl<S> FromRequestParts<S> for AuthenticatedSession
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        let sid = extract_cookie_value(
            parts
                .headers
                .get(COOKIE)
                .and_then(|header| header.to_str().ok()),
            "sid",
        )
        .ok_or_else(ApiError::unauthorized)?;

        let Some(session) = app_state
            .auth_store
            .find_session_by_id(&sid)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        else {
            return Err(ApiError::unauthorized());
        };

        if is_expired(&session) {
            if let Err(error) = app_state.auth_store.delete_session_by_id(&sid).await {
                warn!(%error, "failed to delete expired session");
            }
            return Err(ApiError::unauthorized());
        }

        let Some(user) = app_state
            .auth_store
            .find_user_by_discord_id(&session.user_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        else {
            if let Err(error) = app_state.auth_store.delete_session_by_id(&sid).await {
                warn!(%error, "failed to delete orphaned session");
            }
            return Err(ApiError::unauthorized());
        };

        Ok(Self { sid, session, user })
    }
}

fn is_expired(session: &SessionRecord) -> bool {
    session.expires_at.timestamp_millis() <= mongodb::bson::DateTime::now().timestamp_millis()
}

fn extract_cookie_value(cookie_header: Option<&str>, key: &str) -> Option<String> {
    let header = cookie_header?;

    for pair in header.split(';') {
        let trimmed = pair.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, '=');
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };

        if name == key {
            return Some(value.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use mongodb::bson::DateTime;

    use super::*;

    #[test]
    fn extracts_sid_cookie() {
        let sid = extract_cookie_value(Some("foo=bar; sid=abc123; hello=world"), "sid");
        assert_eq!(sid.as_deref(), Some("abc123"));
    }

    #[test]
    fn ignores_missing_or_malformed_cookie() {
        assert_eq!(extract_cookie_value(None, "sid"), None);
        assert_eq!(extract_cookie_value(Some("foo=bar; sid"), "sid"), None);
        assert_eq!(extract_cookie_value(Some("foo=bar"), "sid"), None);
    }

    #[test]
    fn detects_expired_session() {
        let old = DateTime::from_millis(1);
        let session = SessionRecord {
            session_id: "sid".to_string(),
            user_id: "user".to_string(),
            created_at: old,
            expires_at: old,
        };

        assert!(is_expired(&session));
    }
}
