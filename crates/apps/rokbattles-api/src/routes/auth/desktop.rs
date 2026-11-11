//! Browser OAuth handoff to a loopback listener owned by the desktop app.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use mongodb::bson::DateTime;
use serde::Deserialize;

use super::{cookies, oauth};
use crate::{
    db::{AuthRepository, DesktopLogin, DesktopOAuthState, NewSessionRecord},
    error::ApiError,
    state::AppState,
};

const OAUTH_STATE_TTL_MILLIS: i64 = 60_000;
const SESSION_MAX_AGE_SECONDS: u64 = 30 * 24 * 60 * 60;
const SESSION_TTL_MILLIS: i64 = SESSION_MAX_AGE_SECONDS as i64 * 1000;

fn is_token(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn validate_login(login: &DesktopLogin) -> Result<(), ApiError> {
    if login.port < 1024
        || !is_token(&login.state, 32, 128)
        || !is_token(&login.code_challenge, 43, 43)
    {
        return Err(ApiError::bad_request("invalid desktop login request"));
    }

    Ok(())
}

pub(super) async fn login(
    State(state): State<Arc<AppState>>,
    Query(login): Query<DesktopLogin>,
) -> Result<Response, ApiError> {
    validate_login(&login)?;

    Ok(private(super::begin_discord_login(&state, Some(login)).await?.into_response()))
}

pub(super) fn callback(login: &DesktopLogin, key: &str, value: &str) -> Result<Response, ApiError> {
    // Never accept an arbitrary redirect URI from the request or database.
    validate_login(login)?;

    let mut url = reqwest::Url::parse(&format!("http://127.0.0.1:{}/auth/callback", login.port))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    url.query_pairs_mut().append_pair("state", &login.state).append_pair(key, value);

    Ok(private(Redirect::to(url.as_str()).into_response()))
}

pub(super) async fn authorize(
    state: &AppState,
    login: &DesktopLogin,
    user_id: String,
) -> Result<Response, ApiError> {
    let code = oauth::generate_code_verifier();
    state
        .auth_store
        .insert_desktop_oauth_state(DesktopOAuthState {
            code_hash: oauth::derive_code_challenge(&code),
            code_challenge: login.code_challenge.clone(),
            user_id,
            expires_at: DateTime::from_millis(
                DateTime::now().timestamp_millis() + OAUTH_STATE_TTL_MILLIS,
            ),
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    callback(login, "code", &code)
}

#[derive(Deserialize)]
pub(super) struct ExchangeRequest {
    code: String,
    verifier: String,
}

pub(super) async fn exchange(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExchangeRequest>,
) -> Result<Response, ApiError> {
    exchange_session(state.auth_store.as_ref(), request, DateTime::now()).await
}

async fn exchange_session(
    auth_store: &dyn AuthRepository,
    request: ExchangeRequest,
    now: DateTime,
) -> Result<Response, ApiError> {
    if !is_token(&request.code, 43, 128) || !is_token(&request.verifier, 43, 128) {
        return Err(ApiError::bad_request("invalid desktop authorization code"));
    }

    let oauth_state = auth_store
        .consume_desktop_oauth_state(
            &oauth::derive_code_challenge(&request.code),
            &oauth::derive_code_challenge(&request.verifier),
            now,
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(ApiError::unauthorized)?;

    let now = DateTime::now();
    let sid = oauth::generate_session_id();
    auth_store
        .insert_session(NewSessionRecord {
            session_id: sid.clone(),
            user_id: oauth_state.user_id,
            created_at: now,
            expires_at: DateTime::from_millis(now.timestamp_millis() + SESSION_TTL_MILLIS),
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut response = private(StatusCode::NO_CONTENT.into_response());
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookies::build_set_session_cookie(&sid, SESSION_MAX_AGE_SECONDS)
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid session cookie: {error}")))?,
    );

    Ok(response)
}

fn private(mut response: Response) -> Response {
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));

    response
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use futures::{
        FutureExt,
        future::{BoxFuture, join_all, ready},
    };
    use tokio::sync::Barrier;

    use super::*;
    use crate::{
        auth::extract_cookie_value,
        db::{AuthStoreError, DiscordUserUpsert, OAuthStateRecord, SessionRecord, UserRecord},
    };

    const CODE: &str = "desktop-authorization-code-0123456789abcdefghijklm";
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    const NOW_MILLIS: i64 = 1_700_000_000_000;

    fn request() -> ExchangeRequest {
        ExchangeRequest { code: CODE.to_string(), verifier: VERIFIER.to_string() }
    }

    struct MemoryAuthStore {
        pending: Mutex<Option<DesktopOAuthState>>,
        sessions: Mutex<Vec<NewSessionRecord>>,
        consume_calls: AtomicUsize,
        barrier: Option<Barrier>,
    }

    impl MemoryAuthStore {
        fn new(expires_at: i64) -> Self {
            Self {
                pending: Mutex::new(Some(DesktopOAuthState {
                    code_hash: oauth::derive_code_challenge(CODE),
                    code_challenge: CHALLENGE.to_string(),
                    user_id: "discord-user".to_string(),
                    expires_at: DateTime::from_millis(expires_at),
                })),
                sessions: Mutex::new(Vec::new()),
                consume_calls: AtomicUsize::new(0),
                barrier: None,
            }
        }
    }

    fn unexpected_store_call() -> AuthStoreError {
        AuthStoreError::Database(mongodb::error::Error::custom(
            "unexpected auth repository operation",
        ))
    }

    impl AuthRepository for MemoryAuthStore {
        fn consume_desktop_oauth_state<'a>(
            &'a self,
            code_hash: &'a str,
            code_challenge: &'a str,
            now: DateTime,
        ) -> BoxFuture<'a, Result<Option<DesktopOAuthState>, AuthStoreError>> {
            async move {
                self.consume_calls.fetch_add(1, Ordering::Relaxed);
                if let Some(barrier) = &self.barrier {
                    barrier.wait().await;
                }
                let mut pending = self.pending.lock().expect("pending code lock");
                if pending.as_ref().is_some_and(|record| {
                    record.code_hash == code_hash
                        && record.code_challenge == code_challenge
                        && record.expires_at > now
                }) {
                    Ok(pending.take())
                } else {
                    Ok(None)
                }
            }
            .boxed()
        }

        fn insert_session<'a>(
            &'a self,
            session: NewSessionRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            async move {
                self.sessions.lock().expect("session lock").push(session);
                Ok(())
            }
            .boxed()
        }

        fn insert_desktop_oauth_state(
            &self,
            _state: DesktopOAuthState,
        ) -> BoxFuture<'_, Result<(), AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn find_session_by_id<'a>(
            &'a self,
            _sid: &'a str,
        ) -> BoxFuture<'a, Result<Option<SessionRecord>, AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn delete_session_by_id<'a>(
            &'a self,
            _sid: &'a str,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn find_user_by_discord_id<'a>(
            &'a self,
            _discord_id: &'a str,
        ) -> BoxFuture<'a, Result<Option<UserRecord>, AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn insert_oauth_state<'a>(
            &'a self,
            _state: OAuthStateRecord,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn consume_oauth_state<'a>(
            &'a self,
            _state: &'a str,
        ) -> BoxFuture<'a, Result<Option<OAuthStateRecord>, AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }

        fn upsert_discord_user<'a>(
            &'a self,
            _user: DiscordUserUpsert,
        ) -> BoxFuture<'a, Result<(), AuthStoreError>> {
            ready(Err(unexpected_store_call())).boxed()
        }
    }

    fn login() -> DesktopLogin {
        DesktopLogin { port: 54321, state: "a".repeat(43), code_challenge: "b".repeat(43) }
    }

    #[test]
    fn callback_is_fixed_to_loopback_and_does_not_cache_the_code() {
        let response = callback(&login(), "code", "test_code").expect("callback");
        let url: reqwest::Url =
            response.headers()[header::LOCATION].to_str().expect("header").parse().expect("url");

        assert_eq!(url.origin().ascii_serialization(), "http://127.0.0.1:54321");
        assert_eq!(url.path(), "/auth/callback");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
    }

    #[test]
    fn rejects_privileged_ports_and_invalid_pkce_or_state() {
        let mut request = login();
        validate_login(&request).expect("valid request");

        request.port = 80;
        validate_login(&request).expect_err("invalid request");

        request = login();
        request.state = "too-short".into();
        validate_login(&request).expect_err("invalid request");

        request = login();
        request.code_challenge = "https://example.com/".into();
        validate_login(&request).expect_err("invalid request");
    }

    #[tokio::test]
    async fn valid_exchange_creates_one_bound_session_and_private_cookie_response() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        let response = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("valid exchange");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()[header::REFERRER_POLICY], "no-referrer");
        let cookie = response.headers()[header::SET_COOKIE].to_str().expect("session cookie");
        let sid = extract_cookie_value(Some(cookie), "sid").expect("cookie token");
        let sessions = store.sessions.lock().expect("session lock");
        assert_eq!(sessions.len(), 1);
        let session = sessions.first().expect("issued session");
        assert_eq!(session.session_id, sid);
        assert_eq!(session.user_id, "discord-user");
        assert_eq!(
            session.expires_at.timestamp_millis() - session.created_at.timestamp_millis(),
            30 * 86_400_000
        );
        assert!(store.pending.lock().expect("pending lock").is_none());
    }

    #[tokio::test]
    async fn replayed_code_cannot_issue_a_second_session_or_cookie() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        let first = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("first exchange");
        assert_eq!(first.status(), StatusCode::NO_CONTENT);

        let replay = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect_err("code cannot be replayed")
            .into_response();
        assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
        assert!(!replay.headers().contains_key(header::SET_COOKIE));
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);
    }

    #[tokio::test]
    async fn expired_codes_are_rejected_even_while_the_records_still_exist() {
        for expiry in [NOW_MILLIS - 1, NOW_MILLIS] {
            let store = MemoryAuthStore::new(expiry);
            let response = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
                .await
                .expect_err("expired code")
                .into_response();

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!response.headers().contains_key(header::SET_COOKIE));
            assert!(store.sessions.lock().expect("session lock").is_empty());
            assert!(store.pending.lock().expect("pending lock").is_some());
        }
    }

    #[tokio::test]
    async fn code_one_millisecond_before_expiry_can_be_exchanged() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 1);
        let response = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("unexpired code");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);
    }

    #[tokio::test]
    async fn incorrect_pkce_proof_does_not_burn_the_code_for_its_owner() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        let wrong_proof = ExchangeRequest { verifier: "w".repeat(43), ..request() };
        let rejected = exchange_session(&store, wrong_proof, DateTime::from_millis(NOW_MILLIS))
            .await
            .expect_err("wrong PKCE proof")
            .into_response();

        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
        assert!(!rejected.headers().contains_key(header::SET_COOKIE));
        assert!(store.sessions.lock().expect("session lock").is_empty());
        assert!(store.pending.lock().expect("pending lock").is_some());

        let accepted = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("owner can still exchange");
        assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);
    }

    #[tokio::test]
    async fn presenting_the_pkce_challenge_instead_of_the_verifier_is_rejected() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        let challenge_as_proof = ExchangeRequest { verifier: CHALLENGE.to_string(), ..request() };
        let response =
            exchange_session(&store, challenge_as_proof, DateTime::from_millis(NOW_MILLIS))
                .await
                .expect_err("challenge is not a proof")
                .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(store.sessions.lock().expect("session lock").is_empty());
        let valid = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("original verifier still works");
        assert_eq!(valid.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn stored_code_hash_cannot_be_used_as_the_authorization_code() {
        let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        let hash_as_code =
            ExchangeRequest { code: oauth::derive_code_challenge(CODE), ..request() };
        let response = exchange_session(&store, hash_as_code, DateTime::from_millis(NOW_MILLIS))
            .await
            .expect_err("stored hash is not a code")
            .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(store.sessions.lock().expect("session lock").is_empty());
        let valid = exchange_session(&store, request(), DateTime::from_millis(NOW_MILLIS))
            .await
            .expect("original code still works");
        assert_eq!(valid.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn malformed_exchange_tokens_are_rejected_before_repository_access() {
        for (code, verifier) in [
            ("c".repeat(42), VERIFIER.to_string()),
            ("c".repeat(129), VERIFIER.to_string()),
            ("!".repeat(43), VERIFIER.to_string()),
            (CODE.to_string(), "v".repeat(42)),
            (CODE.to_string(), "v".repeat(129)),
            (CODE.to_string(), "!".repeat(43)),
        ] {
            let store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
            let response = exchange_session(
                &store,
                ExchangeRequest { code, verifier },
                DateTime::from_millis(NOW_MILLIS),
            )
            .await
            .expect_err("malformed tokens")
            .into_response();

            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(!response.headers().contains_key(header::SET_COOKIE));
            assert_eq!(store.consume_calls.load(Ordering::Relaxed), 0);
            assert!(store.sessions.lock().expect("session lock").is_empty());
            assert!(store.pending.lock().expect("pending lock").is_some());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_exchanges_issue_exactly_one_session_and_cookie() {
        const CONTENDERS: usize = 16;
        let mut store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        store.barrier = Some(Barrier::new(CONTENDERS));
        let store = Arc::new(store);
        let tasks = (0..CONTENDERS).map(|_| {
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                exchange_session(store.as_ref(), request(), DateTime::from_millis(NOW_MILLIS))
                    .await
                    .unwrap_or_else(IntoResponse::into_response)
            })
        });
        let responses = tokio::time::timeout(Duration::from_secs(5), join_all(tasks))
            .await
            .expect("all contenders reach consumption");
        let mut successes = 0;
        let mut rejections = 0;
        let mut cookies = 0;
        for response in responses {
            let response = response.expect("exchange task");
            if response.status() == StatusCode::NO_CONTENT {
                successes += 1;
            } else {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
                rejections += 1;
            }
            cookies += usize::from(response.headers().contains_key(header::SET_COOKIE));
        }

        assert_eq!(successes, 1);
        assert_eq!(rejections, CONTENDERS - 1);
        assert_eq!(cookies, 1);
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);
        assert_eq!(store.consume_calls.load(Ordering::Relaxed), CONTENDERS);
        assert!(store.pending.lock().expect("pending lock").is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_incorrect_proofs_cannot_consume_a_valid_owners_code() {
        const CONTENDERS: usize = 16;
        let mut store = MemoryAuthStore::new(NOW_MILLIS + 60_000);
        store.barrier = Some(Barrier::new(CONTENDERS));
        let store = Arc::new(store);
        let tasks = (0..CONTENDERS).map(|index| {
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                let mut proof = request();
                let valid_proof = index % 2 == 0;
                if !valid_proof {
                    proof.verifier = "w".repeat(43);
                }
                let response =
                    exchange_session(store.as_ref(), proof, DateTime::from_millis(NOW_MILLIS))
                        .await
                        .unwrap_or_else(IntoResponse::into_response);
                (valid_proof, response)
            })
        });
        let results = tokio::time::timeout(Duration::from_secs(5), join_all(tasks))
            .await
            .expect("all contenders reach consumption");
        let mut successes = 0;
        for result in results {
            let (valid_proof, response) = result.expect("exchange task");
            if response.status() == StatusCode::NO_CONTENT {
                assert!(valid_proof, "an incorrect PKCE proof issued a session");
                successes += 1;
            } else {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
                assert!(!response.headers().contains_key(header::SET_COOKIE));
            }
        }

        assert_eq!(successes, 1);
        assert_eq!(store.sessions.lock().expect("session lock").len(), 1);
        assert!(store.pending.lock().expect("pending lock").is_none());
    }
}
