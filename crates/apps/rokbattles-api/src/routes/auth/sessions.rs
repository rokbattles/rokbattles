//! Lists and revokes sessions for the authenticated user.
//!
//! Clients reference sessions by their database IDs. Bearer tokens and stored
//! hashes are excluded from responses. The current session is included in the
//! list and marked with `current` so clients can display it separately.

use axum::{
    Json,
    extract::{Path, State},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::{IntoResponse, Response},
};
use mongodb::bson::{DateTime, oid::ObjectId};
use serde::Serialize;

use super::cookies;
use crate::{
    auth::{AuthState, AuthenticatedSession},
    db::SessionPlatform,
    error::ApiError,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDetails {
    id: String,
    platform: SessionPlatform,
    /// Sign-in time as milliseconds since the Unix epoch.
    created_at: i64,
    current: bool,
}

#[derive(Serialize)]
struct SessionsResponse {
    sessions: Vec<SessionDetails>,
}

pub(super) async fn list(
    State(auth): State<AuthState>,
    session: AuthenticatedSession,
) -> Result<Response, ApiError> {
    let current_id = session.session.id;
    let records = auth
        .repository()
        .list_active_sessions(&session.user.discord_id, DateTime::now())
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let sessions = records
        .into_iter()
        .map(|record| SessionDetails {
            current: record.id == current_id,
            id: record.id.to_hex(),
            platform: record.platform,
            created_at: record.created_at.timestamp_millis(),
        })
        .collect();

    Ok(([(CACHE_CONTROL, "no-store")], Json(SessionsResponse { sessions })).into_response())
}

/// Revokes another session owned by the authenticated user.
///
/// The current session must be ended through logout or [`delete_all`]. Missing
/// sessions and sessions owned by another user both return a not-found response.
pub(super) async fn delete_one(
    State(auth): State<AuthState>,
    session: AuthenticatedSession,
    Path(session_id): Path<String>,
) -> Result<Response, ApiError> {
    let session_id = ObjectId::parse_str(&session_id)
        .map_err(|_error| ApiError::bad_request("invalid session ID"))?;

    if session_id == session.session.id {
        return Err(ApiError::bad_request("use logout to end the current session"));
    }

    let deleted = auth
        .repository()
        .delete_user_session(&session.user.discord_id, session_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if !deleted {
        return Err(ApiError::not_found("session not found"));
    }

    Ok(([(CACHE_CONTROL, "no-store")], StatusCode::NO_CONTENT).into_response())
}

/// Revokes all of the user's sessions and clears the browser's session cookie.
///
/// The cookie is cleared only after the session store confirms deletion.
pub(super) async fn delete_all(
    State(auth): State<AuthState>,
    session: AuthenticatedSession,
) -> Result<Response, ApiError> {
    auth.repository()
        .delete_user_sessions(&session.user.discord_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((
        [(CACHE_CONTROL, "no-store"), (SET_COOKIE, &cookies::build_clear_session_cookie())],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Method, Request},
        routing::{delete, get},
    };
    use futures::{
        FutureExt,
        future::{BoxFuture, ready},
    };
    use tower::ServiceExt;

    use super::*;
    use crate::db::{
        AuthRepository, AuthStoreError, DesktopOAuthState, DiscordUserUpsert, ListedSessionRecord,
        NewSessionRecord, OAuthStateRecord, SessionRecord, UserRecord,
    };

    #[derive(Clone)]
    struct MemorySession {
        record: SessionRecord,
        platform: SessionPlatform,
    }

    #[derive(Default)]
    struct MemoryAuthStore {
        sessions: Mutex<BTreeMap<String, MemorySession>>,
        fail_management: AtomicBool,
    }

    impl MemoryAuthStore {
        fn insert(&self, sid: &str, owner: &str, platform: SessionPlatform, expires_at: i64) {
            self.sessions.lock().expect("session lock").insert(
                sid.to_owned(),
                MemorySession {
                    record: SessionRecord {
                        id: ObjectId::new(),
                        user_id: owner.to_owned(),
                        created_at: DateTime::from_millis(1_700_000_000_000),
                        expires_at: DateTime::from_millis(expires_at),
                    },
                    platform,
                },
            );
        }
    }

    fn store_error() -> AuthStoreError {
        AuthStoreError::Database(mongodb::error::Error::custom("test store failure"))
    }

    impl AuthRepository for MemoryAuthStore {
        fn find_session_by_id<'a>(
            &'a self,
            sid: &'a str,
        ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
            ready(Ok(self
                .sessions
                .lock()
                .expect("session lock")
                .get(sid)
                .map(|s| s.record.clone())))
            .boxed()
        }

        fn find_user_by_discord_id<'a>(
            &'a self,
            id: &'a str,
        ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
            ready(Ok(Some(UserRecord {
                discord_id: id.to_owned(),
                email: "test@example.com".to_owned(),
                roles: Default::default(),
            })))
            .boxed()
        }

        fn delete_session_by_id<'a>(
            &'a self,
            sid: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            self.sessions.lock().expect("session lock").remove(sid);

            ready(Ok(())).boxed()
        }

        fn list_active_sessions<'a>(
            &'a self,
            user_id: &'a str,
            now: DateTime,
        ) -> BoxFuture<'a, Result<Vec<ListedSessionRecord>, AuthStoreError>> {
            if self.fail_management.load(Ordering::Relaxed) {
                return ready(Err(store_error())).boxed();
            }

            let mut sessions: Vec<_> = self
                .sessions
                .lock()
                .expect("session lock")
                .iter()
                .filter(|(_, s)| s.record.user_id == user_id && s.record.expires_at > now)
                .map(|(_, s)| ListedSessionRecord {
                    id: s.record.id,
                    platform: s.platform,
                    created_at: s.record.created_at,
                })
                .collect();

            sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));

            ready(Ok(sessions)).boxed()
        }

        fn delete_user_session<'a>(
            &'a self,
            user_id: &'a str,
            session_id: ObjectId,
        ) -> BoxFuture<'a, Result<bool, AuthStoreError>> {
            if self.fail_management.load(Ordering::Relaxed) {
                return ready(Err(store_error())).boxed();
            }

            let mut sessions = self.sessions.lock().expect("session lock");
            let sid = sessions
                .iter()
                .find(|(_, s)| s.record.user_id == user_id && s.record.id == session_id)
                .map(|(sid, _)| sid.clone());

            ready(Ok(sid.is_some_and(|sid| sessions.remove(&sid).is_some()))).boxed()
        }

        fn delete_user_sessions<'a>(
            &'a self,
            user_id: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            if self.fail_management.load(Ordering::Relaxed) {
                return ready(Err(store_error())).boxed();
            }

            self.sessions.lock().expect("session lock").retain(|_, s| s.record.user_id != user_id);

            ready(Ok(())).boxed()
        }

        fn delete_account<'a>(
            &'a self,
            _user_id: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn insert_session<'a>(
            &'a self,
            record: NewSessionRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            self.sessions.lock().expect("session lock").insert(
                record.session_id,
                MemorySession {
                    record: SessionRecord {
                        id: ObjectId::new(),
                        user_id: record.user_id,
                        created_at: record.created_at,
                        expires_at: record.expires_at,
                    },
                    platform: record.platform,
                },
            );

            ready(Ok(())).boxed()
        }

        fn insert_desktop_oauth_state(
            &self,
            _s: DesktopOAuthState,
        ) -> BoxFuture<'_, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn consume_desktop_oauth_state<'a>(
            &'a self,
            _code: &'a str,
            _challenge: &'a str,
            _now: DateTime,
        ) -> BoxFuture<'a, Result<Option<DesktopOAuthState>, AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn insert_oauth_state<'a>(
            &'a self,
            _s: OAuthStateRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn consume_oauth_state<'a>(
            &'a self,
            _state: &'a str,
        ) -> BoxFuture<'a, Result<Option<OAuthStateRecord>, AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn upsert_discord_user<'a>(
            &'a self,
            _user: DiscordUserUpsert,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }
    }

    fn id(store: &MemoryAuthStore, sid: &str) -> String {
        store.sessions.lock().expect("session lock")[sid].record.id.to_hex()
    }

    fn setup() -> (Arc<MemoryAuthStore>, Router) {
        let store = Arc::new(MemoryAuthStore::default());
        store.insert("current-token", "alice", SessionPlatform::Browser, i64::MAX);
        store.insert("desktop-token", "alice", SessionPlatform::Desktop, i64::MAX);
        store.insert("legacy-token", "alice", SessionPlatform::Unknown, i64::MAX);
        store.insert("expired-token", "alice", SessionPlatform::Browser, 0);
        store.insert("foreign-token", "bob", SessionPlatform::Browser, i64::MAX);

        let app = Router::new()
            .route("/sessions", get(list).delete(delete_all))
            .route("/sessions/{session_id}", delete(delete_one))
            .with_state(AuthState::new(store.clone()));

        (store, app)
    }

    async fn request(app: &Router, method: Method, path: &str, sid: Option<&str>) -> Response {
        let mut request = Request::builder().method(method).uri(path);

        if let Some(sid) = sid {
            request = request.header("cookie", format!("sid={sid}"));
        }

        app.clone().oneshot(request.body(Body::empty()).expect("request")).await.expect("response")
    }

    #[tokio::test]
    async fn list_returns_only_active_owned_sessions_and_marks_current_without_exposing_tokens() {
        let (store, app) = setup();
        let response = request(&app, Method::GET, "/sessions", Some("current-token")).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");

        let bytes = to_bytes(response.into_body(), 8192).await.expect("body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        let rows = body["sessions"].as_array().expect("sessions");

        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|s| s["current"] == true).count(), 1);

        let current = rows.iter().find(|s| s["current"] == true).expect("current");

        assert_eq!(current["platform"], "browser");
        assert_eq!(current["id"], id(&store, "current-token"));
        assert_eq!(current["createdAt"], 1_700_000_000_000_i64);
        assert!(rows.iter().any(|s| s["platform"] == "desktop"));
        assert!(rows.iter().any(|s| s["platform"] == "unknown"));

        let text = String::from_utf8(bytes.to_vec()).expect("UTF8");

        for token in
            ["current-token", "desktop-token", "legacy-token", "foreign-token", "expired-token"]
        {
            assert!(!text.contains(token));
        }
    }

    #[tokio::test]
    async fn all_management_operations_require_a_live_authenticated_session() {
        let (_, app) = setup();

        for sid in [None, Some("missing-token"), Some("expired-token")] {
            for (method, path) in [
                (Method::GET, "/sessions"),
                (Method::DELETE, "/sessions"),
                (Method::DELETE, "/sessions/000000000000000000000000"),
            ] {
                assert_eq!(
                    request(&app, method, path, sid).await.status(),
                    StatusCode::UNAUTHORIZED
                );
            }
        }
    }

    #[tokio::test]
    async fn revoking_another_session_invalidates_it_but_keeps_the_current_session() {
        let (store, app) = setup();
        let path = format!("/sessions/{}", id(&store, "desktop-token"));
        let response = request(&app, Method::DELETE, &path, Some("current-token")).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(!response.headers().contains_key(SET_COOKIE));
        assert_eq!(
            request(&app, Method::GET, "/sessions", Some("desktop-token")).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(&app, Method::GET, "/sessions", Some("current-token")).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn revocation_cannot_target_another_user_or_the_current_session() {
        let (store, app) = setup();

        for sid in ["foreign-token", "missing-token"] {
            let path = format!(
                "/sessions/{}",
                if sid == "missing-token" { ObjectId::new().to_hex() } else { id(&store, sid) }
            );

            assert_eq!(
                request(&app, Method::DELETE, &path, Some("current-token")).await.status(),
                StatusCode::NOT_FOUND
            );
        }

        let path = format!("/sessions/{}", id(&store, "current-token"));

        assert_eq!(
            request(&app, Method::DELETE, &path, Some("current-token")).await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            request(&app, Method::GET, "/sessions", Some("foreign-token")).await.status(),
            StatusCode::OK
        );
        assert_eq!(
            request(&app, Method::GET, "/sessions", Some("current-token")).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn logout_all_invalidates_every_owned_session_including_current_and_clears_cookie() {
        let (store, app) = setup();
        let response = request(&app, Method::DELETE, "/sessions", Some("current-token")).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()[SET_COOKIE], cookies::build_clear_session_cookie());
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);

        for sid in ["current-token", "desktop-token", "legacy-token"] {
            assert_eq!(
                request(&app, Method::GET, "/sessions", Some(sid)).await.status(),
                StatusCode::UNAUTHORIZED
            );
        }

        assert_eq!(
            request(&app, Method::GET, "/sessions", Some("foreign-token")).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn store_failures_do_not_report_success_or_clear_the_cookie() {
        let (store, app) = setup();
        store.fail_management.store(true, Ordering::Relaxed);

        for (method, path) in [
            (Method::GET, "/sessions"),
            (Method::DELETE, "/sessions"),
            (Method::DELETE, "/sessions/000000000000000000000000"),
        ] {
            let response = request(&app, method, path, Some("current-token")).await;

            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert!(!response.headers().contains_key(SET_COOKIE));
        }

        assert_eq!(store.sessions.lock().expect("session lock").len(), 5);
    }

    #[tokio::test]
    async fn browser_sign_in_issues_a_browser_session_with_existing_cookie_and_expiry() {
        let store = MemoryAuthStore::default();
        let now = DateTime::now();
        let response = super::super::create_browser_session(&store, "alice".to_owned(), now)
            .await
            .expect("browser sign-in");

        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        let cookie = response.headers()[SET_COOKIE].to_str().expect("cookie");
        let sid = crate::auth::extract_cookie_value(Some(cookie), "sid").expect("session ID");
        let sessions = store.sessions.lock().expect("session lock");
        let session = &sessions[&sid];

        assert_eq!(session.platform, SessionPlatform::Browser);
        assert_eq!(session.record.user_id, "alice");
        assert_eq!(session.record.created_at, now);
        assert_eq!(
            session.record.expires_at.timestamp_millis() - now.timestamp_millis(),
            super::super::SESSION_TTL_MILLIS
        );
        assert!(cookie.contains("HttpOnly; Secure; SameSite=Lax"));
    }

    #[tokio::test]
    async fn malformed_session_ids_are_rejected_without_deleting_sessions() {
        let (store, app) = setup();
        let response =
            request(&app, Method::DELETE, "/sessions/not-an-object-id", Some("current-token"))
                .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(store.sessions.lock().expect("session lock").len(), 5);
    }
}
