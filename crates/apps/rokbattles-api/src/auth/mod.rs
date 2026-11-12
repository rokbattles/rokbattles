//! Session authentication and role checks.
//!
//! [`AuthenticatedSession`] validates the `sid` cookie and loads the current user
//! for each request. [`RequireRole`] also checks role membership before the handler
//! runs. Roles do not replace resource ownership checks.
//!
//! Additional roles are stored in `users.roles`, for example `["premium",
//! "content_creator"]`. [`Role::User`] is implicit, and [`Role::Admin`] does not
//! imply any other role. `/auth/me` returns the effective roles, including `User`.
//! Missing or null roles give a user no additional memberships. Invalid role data
//! causes authentication to return an internal error.
//!
//! # Examples
//!
//! Requiring a role for a route:
//!
//! ```
//! use rokbattles_api::auth::{RequireRole, roles};
//!
//! async fn premium_feature(_session: RequireRole<roles::Premium>) -> &'static str {
//!     "Premium feature enabled"
//! }
//! ```
//!
//! Accepting either developer program or administrator membership in a function:
//!
//! ```
//! use rokbattles_api::{auth::{AuthenticatedSession, Role}, error::ApiError};
//!
//! fn check_developer_access(session: &AuthenticatedSession) -> Result<(), ApiError> {
//!     session.require_any_role(&[Role::DeveloperProgram, Role::Admin])
//! }
//! ```

use std::{marker::PhantomData, ops::Deref, sync::Arc};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::COOKIE, request::Parts},
};
use tracing::warn;

use crate::{
    db::{AuthRepository, SessionRecord, UserRecord},
    error::ApiError,
    state::AppState,
};

pub mod roles;

pub use roles::{Role, Roles};

/// Shared repository used by authentication extractors.
///
/// Extracted from [`AppState`] through [`FromRef`]. Routers can also supply this
/// state directly when they need only authentication.
#[derive(Clone)]
pub struct AuthState(Arc<dyn AuthRepository>);

impl AuthState {
    /// Creates authentication state backed by the given repository.
    pub fn new(repository: Arc<dyn AuthRepository>) -> Self {
        Self(repository)
    }
}

impl FromRef<Arc<AppState>> for AuthState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        Self::new(state.auth_store.clone())
    }
}

/// A validated session and the user loaded for the current request.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub sid: String,
    pub session: SessionRecord,
    pub user: UserRecord,
}

impl AuthenticatedSession {
    /// Returns `true` if the current user has the given role.
    pub fn has_role(&self, role: Role) -> bool {
        self.user.roles.contains(role)
    }

    /// Checks that the current user has the given role.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Forbidden`] if the role is absent.
    pub fn require_role(&self, role: Role) -> Result<(), ApiError> {
        self.require_any_role(&[role])
    }

    /// Checks that the current user has at least one of the given roles.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Forbidden`] if no role matches or the slice is empty.
    pub fn require_any_role(&self, roles: &[Role]) -> Result<(), ApiError> {
        if roles.iter().any(|role| self.has_role(*role)) {
            Ok(())
        } else {
            Err(ApiError::Forbidden("insufficient-role".to_string()))
        }
    }

    /// Checks that the current user has every given role.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError::Forbidden`] if any role is absent or the slice is empty.
    pub fn require_all_roles(&self, roles: &[Role]) -> Result<(), ApiError> {
        if !roles.is_empty() && roles.iter().all(|role| self.has_role(*role)) {
            Ok(())
        } else {
            Err(ApiError::Forbidden("insufficient-role".to_string()))
        }
    }
}

/// An extractor that authenticates a session and checks a required role.
///
/// Use a marker such as [`roles::Premium`] as the type parameter. The extractor
/// dereferences to [`AuthenticatedSession`] so handlers can access the current user.
/// See the [module examples](self) for usage.
///
/// Requests without a valid session are rejected with `401 Unauthorized`. A valid
/// session without the required role is rejected with `403 Forbidden`. Repository
/// errors are returned as `500 Internal Server Error`.
pub struct RequireRole<R> {
    session: AuthenticatedSession,
    required: PhantomData<R>,
}

impl<R> RequireRole<R> {
    /// Consumes the extractor, returning the authenticated session.
    pub fn into_inner(self) -> AuthenticatedSession {
        self.session
    }
}

impl<R> Deref for RequireRole<R> {
    type Target = AuthenticatedSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl<S, R> FromRequestParts<S> for RequireRole<R>
where
    AuthState: FromRef<S>,
    S: Send + Sync,
    R: roles::RequiredRole,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = AuthenticatedSession::from_request_parts(parts, state).await?;
        session.require_role(R::ROLE)?;

        Ok(Self { session, required: PhantomData })
    }
}

impl<S> FromRequestParts<S> for AuthenticatedSession
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthState(auth_store) = AuthState::from_ref(state);

        let sid = extract_cookie_value(
            parts.headers.get(COOKIE).and_then(|header| header.to_str().ok()),
            "sid",
        )
        .ok_or_else(ApiError::unauthorized)?;

        let Some(session) = auth_store
            .find_session_by_id(&sid)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        else {
            return Err(ApiError::unauthorized());
        };

        if is_expired(&session) {
            if let Err(error) = auth_store.delete_session_by_id(&sid).await {
                warn!(%error, "failed to delete expired session");
            }

            return Err(ApiError::unauthorized());
        }

        let Some(user) = auth_store
            .find_user_by_discord_id(&session.user_id)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?
        else {
            if let Err(error) = auth_store.delete_session_by_id(&sid).await {
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

pub(crate) fn extract_cookie_value(cookie_header: Option<&str>, key: &str) -> Option<String> {
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
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use futures::{
        FutureExt,
        future::{BoxFuture, ready},
    };
    use mongodb::bson::DateTime;
    use tower::ServiceExt;

    use super::*;
    use crate::db::{
        AuthRepository, AuthStoreError, DesktopOAuthState, DiscordUserUpsert, NewSessionRecord,
        OAuthStateRecord,
    };

    const PRIVILEGED_ROLES: [Role; 4] =
        [Role::Premium, Role::ContentCreator, Role::DeveloperProgram, Role::Admin];

    fn session(roles: Roles) -> AuthenticatedSession {
        AuthenticatedSession {
            sid: "test-session".to_string(),
            session: SessionRecord {
                user_id: "user".to_string(),
                created_at: DateTime::from_millis(0),
                expires_at: DateTime::from_millis(i64::MAX),
            },
            user: UserRecord {
                discord_id: "user".to_string(),
                email: "user@example.com".to_string(),
                roles,
            },
        }
    }

    fn roles_for_mask(mask: usize) -> Roles {
        PRIVILEGED_ROLES
            .into_iter()
            .enumerate()
            .filter_map(|(index, role)| (mask & (1 << index) != 0).then_some(role))
            .collect()
    }

    #[test]
    fn all_role_combinations_have_exact_membership_without_admin_override() {
        for mask in 0..16 {
            let session = session(roles_for_mask(mask));
            session.require_role(Role::User).expect("implicit User");

            for (index, role) in PRIVILEGED_ROLES.into_iter().enumerate() {
                assert_eq!(session.has_role(role), mask & (1 << index) != 0);
                assert_eq!(session.require_role(role).is_ok(), mask & (1 << index) != 0);
            }

            assert_eq!(session.require_any_role(&PRIVILEGED_ROLES).is_ok(), mask != 0);
            assert_eq!(session.require_all_roles(&PRIVILEGED_ROLES).is_ok(), mask == 15);

            for first in PRIVILEGED_ROLES {
                for second in PRIVILEGED_ROLES {
                    let required = [first, second];

                    assert_eq!(
                        session.require_any_role(&required).is_ok(),
                        session.has_role(first) || session.has_role(second)
                    );
                    assert_eq!(
                        session.require_all_roles(&required).is_ok(),
                        session.has_role(first) && session.has_role(second)
                    );
                }
            }
        }
    }

    #[test]
    fn empty_role_requirements_deny_access() {
        let session = session(PRIVILEGED_ROLES.into_iter().collect());

        assert!(matches!(session.require_any_role(&[]), Err(ApiError::Forbidden(_))));
        assert!(matches!(session.require_all_roles(&[]), Err(ApiError::Forbidden(_))));
    }

    struct MemoryAuthStore {
        session: Mutex<Option<SessionRecord>>,
        user: Mutex<Option<UserRecord>>,
        fail_reads: AtomicBool,
    }

    impl MemoryAuthStore {
        fn new() -> Self {
            let principal = session(Roles::default());

            Self {
                session: Mutex::new(Some(principal.session)),
                user: Mutex::new(Some(principal.user)),
                fail_reads: AtomicBool::new(false),
            }
        }

        fn set_roles(&self, roles: Roles) {
            self.user.lock().expect("user lock").as_mut().expect("user").roles = roles;
        }
    }

    fn store_error() -> AuthStoreError {
        AuthStoreError::Database(mongodb::error::Error::custom("auth store failure"))
    }

    impl AuthRepository for MemoryAuthStore {
        fn find_session_by_id<'a>(
            &'a self,
            sid: &'a str,
        ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
            ready(if self.fail_reads.load(Ordering::Relaxed) {
                Err(store_error())
            } else if sid == "test-session" {
                Ok(self.session.lock().expect("session lock").clone())
            } else {
                Ok(None)
            })
            .boxed()
        }

        fn find_user_by_discord_id<'a>(
            &'a self,
            _id: &'a str,
        ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
            ready(Ok(self.user.lock().expect("user lock").clone())).boxed()
        }

        fn delete_session_by_id<'a>(
            &'a self,
            _sid: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            self.session.lock().expect("session lock").take();

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

        fn upsert_discord_user<'a>(
            &'a self,
            _user: DiscordUserUpsert,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }

        fn insert_session<'a>(
            &'a self,
            _session: NewSessionRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(store_error())).boxed()
        }
    }

    fn router(store: Arc<MemoryAuthStore>) -> Router {
        Router::new()
            .route("/user", get(|_: RequireRole<roles::User>| async { StatusCode::NO_CONTENT }))
            .route(
                "/premium",
                get(|_: RequireRole<roles::Premium>| async { StatusCode::NO_CONTENT }),
            )
            .route(
                "/creator",
                get(|_: RequireRole<roles::ContentCreator>| async { StatusCode::NO_CONTENT }),
            )
            .route(
                "/developer",
                get(|_: RequireRole<roles::DeveloperProgram>| async { StatusCode::NO_CONTENT }),
            )
            .route("/admin", get(|_: RequireRole<roles::Admin>| async { StatusCode::NO_CONTENT }))
            .route("/existing", get(|_: AuthenticatedSession| async { StatusCode::NO_CONTENT }))
            .with_state(AuthState::new(store))
    }

    async fn status(router: &Router, path: &str, cookie: Option<&str>) -> StatusCode {
        let mut request = Request::builder().uri(path);

        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }

        router
            .clone()
            .oneshot(request.body(Body::empty()).expect("request"))
            .await
            .expect("response")
            .status()
    }

    #[tokio::test]
    async fn guards_enforce_every_role_combination_and_preserve_auth_only_routes() {
        let store = Arc::new(MemoryAuthStore::new());
        let router = router(store.clone());

        for mask in 0..16 {
            store.set_roles(roles_for_mask(mask));

            for (index, path) in
                ["/premium", "/creator", "/developer", "/admin"].into_iter().enumerate()
            {
                let expected = if mask & (1 << index) != 0 {
                    StatusCode::NO_CONTENT
                } else {
                    StatusCode::FORBIDDEN
                };

                assert_eq!(status(&router, path, Some("sid=test-session")).await, expected);
            }

            for path in ["/user", "/existing"] {
                assert_eq!(
                    status(&router, path, Some("sid=test-session")).await,
                    StatusCode::NO_CONTENT
                );
            }
        }
    }

    #[tokio::test]
    async fn role_grants_and_revocations_apply_to_the_next_request() {
        let store = Arc::new(MemoryAuthStore::new());
        let router = router(store.clone());

        assert_eq!(
            status(&router, "/premium", Some("sid=test-session")).await,
            StatusCode::FORBIDDEN
        );

        store.set_roles([Role::Premium].into_iter().collect());
        assert_eq!(
            status(&router, "/premium", Some("sid=test-session")).await,
            StatusCode::NO_CONTENT
        );

        store.set_roles(Roles::default());
        assert_eq!(
            status(&router, "/premium", Some("sid=test-session")).await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn guards_reject_missing_and_invalid_sessions() {
        let store = Arc::new(MemoryAuthStore::new());
        let router = router(store);

        for cookie in [None, Some("sid=wrong"), Some("unrelated=value")] {
            assert_eq!(status(&router, "/user", cookie).await, StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn expired_sessions_are_deleted_and_cannot_be_reused() {
        let store = Arc::new(MemoryAuthStore::new());
        let router = router(store.clone());

        {
            let mut session = store.session.lock().expect("session lock");
            session.as_mut().expect("session").expires_at = DateTime::from_millis(0);
        }

        assert_eq!(
            status(&router, "/user", Some("sid=test-session")).await,
            StatusCode::UNAUTHORIZED
        );
        assert!(store.session.lock().expect("session lock").is_none());

        assert_eq!(
            status(&router, "/user", Some("sid=test-session")).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn guards_reject_sessions_without_a_user() {
        let store = Arc::new(MemoryAuthStore::new());
        let router = router(store.clone());

        store.user.lock().expect("user lock").take();

        assert_eq!(
            status(&router, "/user", Some("sid=test-session")).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn repository_errors_do_not_grant_access() {
        let store = Arc::new(MemoryAuthStore::new());
        store.fail_reads.store(true, Ordering::Relaxed);
        let router = router(store);

        assert_eq!(
            status(&router, "/user", Some("sid=test-session")).await,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn service_bearer_tokens_cannot_satisfy_user_role_guards() {
        let router = router(Arc::new(MemoryAuthStore::new()));
        let request = Request::builder()
            .uri("/user")
            .header("authorization", "Bearer service-token")
            .body(Body::empty())
            .expect("request");

        assert_eq!(
            router.oneshot(request).await.expect("response").status(),
            StatusCode::UNAUTHORIZED
        );
    }

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
        let session =
            SessionRecord { user_id: "user".to_string(), created_at: old, expires_at: old };

        assert!(is_expired(&session));
    }
}
