//! Permanent deletion of the authenticated user's account.

use axum::{
    Json,
    extract::State,
    http::{
        StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::cookies;
use crate::{
    auth::{AuthState, AuthenticatedSession},
    error::ApiError,
};

/// Explicit confirmation supplied after the account deletion dialog is accepted.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteAccountRequest {
    confirmed: bool,
}

/// Deletes the authenticated account and clears its browser session cookie.
///
/// The account ID comes from authentication. The request cannot select another
/// user. The cookie is cleared only after all three deletion steps succeed.
pub(super) async fn delete(
    State(auth): State<AuthState>,
    session: AuthenticatedSession,
    Json(request): Json<DeleteAccountRequest>,
) -> Result<Response, ApiError> {
    if !request.confirmed {
        return Err(ApiError::bad_request("account deletion must be confirmed"));
    }

    let operation = auth.account_operations().lock(&session.user.discord_id).await;
    session.ensure_active(auth.repository()).await?;

    auth.repository()
        .delete_account(&session.user.discord_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    operation.mark_deleted();

    Ok((
        [(CACHE_CONTROL, "no-store"), (SET_COOKIE, &cookies::build_clear_session_cookie())],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        Router,
        body::Body,
        http::{Method, Request},
        routing::delete,
    };
    use futures::{
        FutureExt,
        future::{BoxFuture, ready},
    };
    use mongodb::bson::{DateTime, oid::ObjectId};
    use tower::ServiceExt;

    use super::*;
    use crate::db::{
        AuthRepository, AuthStoreError, DesktopOAuthState, DiscordUserUpsert, ListedSessionRecord,
        NewSessionRecord, OAuthStateRecord, SessionRecord, UserRecord,
    };

    #[derive(Default)]
    struct TestAuthStore {
        deleted: Mutex<Vec<String>>,
        fail_deletion: bool,
    }

    fn store_error() -> AuthStoreError {
        AuthStoreError::Database(mongodb::error::Error::custom("test store failure"))
    }

    impl AuthRepository for TestAuthStore {
        fn find_session_by_id<'a>(
            &'a self,
            sid: &'a str,
        ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
            let session = match sid {
                "alice" | "bob" | "expired" => Some(SessionRecord {
                    id: ObjectId::new(),
                    user_id: if sid == "bob" { "bob" } else { "alice" }.to_owned(),
                    created_at: DateTime::from_millis(0),
                    expires_at: DateTime::from_millis(if sid == "expired" { 0 } else { i64::MAX }),
                }),
                _ => None,
            };

            ready(Ok(session)).boxed()
        }

        fn find_user_by_discord_id<'a>(
            &'a self,
            user_id: &'a str,
        ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
            let deleted = self.deleted.lock().expect("deletion lock");
            let user = (!deleted.iter().any(|id| id == user_id)).then(|| UserRecord {
                discord_id: user_id.to_owned(),
                email: "test@example.com".to_owned(),
                roles: Default::default(),
            });

            ready(Ok(user)).boxed()
        }

        fn delete_account<'a>(
            &'a self,
            user_id: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            if self.fail_deletion {
                return ready(Err(store_error())).boxed();
            }

            self.deleted.lock().expect("deletion lock").push(user_id.to_owned());

            ready(Ok(())).boxed()
        }

        fn insert_desktop_oauth_state(
            &self,
            _state: DesktopOAuthState,
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
            _state: OAuthStateRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn consume_oauth_state<'a>(
            &'a self,
            _state: &'a str,
        ) -> BoxFuture<'a, Result<Option<OAuthStateRecord>, AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn delete_session_by_id<'a>(
            &'a self,
            _sid: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn list_active_sessions<'a>(
            &'a self,
            _user_id: &'a str,
            _now: DateTime,
        ) -> BoxFuture<'a, Result<Vec<ListedSessionRecord>, AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn delete_user_session<'a>(
            &'a self,
            _user_id: &'a str,
            _session_id: ObjectId,
        ) -> BoxFuture<'a, Result<bool, AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn delete_user_sessions<'a>(
            &'a self,
            _user_id: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn upsert_discord_user<'a>(
            &'a self,
            _user: DiscordUserUpsert,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn insert_session<'a>(
            &'a self,
            _record: NewSessionRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }
    }

    fn router(store: Arc<TestAuthStore>) -> Router {
        Router::new().route("/account", delete(super::delete)).with_state(AuthState::new(store))
    }

    async fn request(app: &Router, sid: Option<&str>, body: &str) -> Response {
        let mut request = Request::builder()
            .method(Method::DELETE)
            .uri("/account")
            .header("content-type", "application/json");

        if let Some(sid) = sid {
            request = request.header("cookie", format!("sid={sid}"));
        }

        app.clone()
            .oneshot(request.body(Body::from(body.to_owned())).expect("request"))
            .await
            .expect("response")
    }

    #[tokio::test]
    async fn deletion_requires_an_authenticated_unexpired_session() {
        let store = Arc::new(TestAuthStore::default());
        let app = router(store.clone());

        for sid in [None, Some("missing"), Some("expired")] {
            let response = request(&app, sid, r#"{"confirmed":true}"#).await;

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!response.headers().contains_key(SET_COOKIE));
        }

        assert!(store.deleted.lock().expect("deletion lock").is_empty());
    }

    #[tokio::test]
    async fn deletion_requires_confirmation_and_rejects_target_user_input() {
        let store = Arc::new(TestAuthStore::default());
        let app = router(store.clone());

        for body in [r#"{"confirmed":false}"#, "{}", r#"{"confirmed":true,"userId":"bob"}"#] {
            let response = request(&app, Some("alice"), body).await;

            assert!(response.status().is_client_error());
            assert!(!response.headers().contains_key(SET_COOKIE));
        }

        assert!(store.deleted.lock().expect("deletion lock").is_empty());
    }

    #[tokio::test]
    async fn successful_deletion_targets_the_authenticated_user_and_clears_the_cookie() {
        let store = Arc::new(TestAuthStore::default());
        let app = router(store.clone());
        let response = request(&app, Some("alice"), r#"{"confirmed":true}"#).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[SET_COOKIE], cookies::build_clear_session_cookie());
        assert_eq!(*store.deleted.lock().expect("deletion lock"), ["alice"]);

        let response = request(&app, Some("alice"), r#"{"confirmed":true}"#).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(store.find_user_by_discord_id("bob").await.expect("other user").is_some());
    }

    #[tokio::test]
    async fn failed_deletion_returns_an_error_without_clearing_the_cookie() {
        let store = Arc::new(TestAuthStore { fail_deletion: true, ..Default::default() });
        let app = router(store.clone());
        let response = request(&app, Some("alice"), r#"{"confirmed":true}"#).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!response.headers().contains_key(SET_COOKIE));
        assert!(store.deleted.lock().expect("deletion lock").is_empty());
    }
}
