//! In-memory repositories and a router fixture for OIDC tests.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request as HttpRequest, StatusCode, header},
    response::Response,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use futures::{
    FutureExt,
    future::{BoxFuture, ready},
};
use mongodb::{
    Client,
    bson::{DateTime, oid::ObjectId},
    error::Result as MongoResult,
};
use reqwest::Url;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tower::ServiceExt;

use super::{
    Oidc, OidcConfig, after, hash,
    store::{BrowserBinding, Code, Grant, OidcRepository, RefreshFamily, Request},
};
use crate::{
    auth::{Role, Roles},
    db::{
        AuthRepository, AuthStoreError, DesktopOAuthState, DeveloperApp, DeveloperAppSettings,
        DiscordUserUpsert, GameLocalizationStore, GameQueryStore, ListedSessionRecord,
        NewSessionRecord, OAuthStateRecord, ReportsStore, SessionRecord, TerritoryPlannerV2Store,
        UserRecord,
    },
    rate_limit::{RateLimitMode, RateLimiter},
    state::{AppState, DiscordOAuthConfig},
};

pub(super) const ORIGIN: &str = "https://rokbattles.example";
pub(super) const CALLBACK: &str = "https://client.example/callback?source=fixture";
pub(super) const USER: &str = "424242424242424242";
const BOB: &str = "525252525252525252";
const APP_OWNER: &str = "owner";
pub(super) const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

struct Auth {
    alice: Mutex<Option<SessionRecord>>,
    alice_sid: Mutex<String>,
    bob: SessionRecord,
    upstream: Mutex<Vec<OAuthStateRecord>>,
    owner_in_program: AtomicBool,
}

type AuthResult<'a, T> = BoxFuture<'a, Result<T, AuthStoreError>>;

impl AuthRepository for Auth {
    fn find_session_by_id<'a>(&'a self, sid: &'a str) -> AuthResult<'a, Option<SessionRecord>> {
        let alice_sid = self.alice_sid.lock().expect("session cookie");
        let record = match sid {
            sid if sid == alice_sid.as_str() => self.alice.lock().expect("sessions").clone(),
            "bob" => Some(self.bob.clone()),
            _ => None,
        };
        ready(Ok(record)).boxed()
    }

    fn active_session_by_record_id<'a>(
        &'a self,
        user: &'a str,
        id: ObjectId,
        now: DateTime,
    ) -> AuthResult<'a, Option<SessionRecord>> {
        let record = self.alice.lock().expect("sessions").clone().filter(|session| {
            session.user_id == user && session.id == id && session.expires_at > now
        });
        ready(Ok(record)).boxed()
    }

    fn find_user_by_discord_id<'a>(&'a self, id: &'a str) -> AuthResult<'a, Option<UserRecord>> {
        let record = match id {
            USER | BOB => Some(UserRecord {
                discord_id: id.to_owned(),
                email: "private@example.com".to_owned(),
                roles: Roles::default(),
            }),
            APP_OWNER => Some(UserRecord {
                discord_id: id.to_owned(),
                email: String::new(),
                roles: self
                    .owner_in_program
                    .load(Ordering::Relaxed)
                    .then_some(Role::DeveloperProgram)
                    .into_iter()
                    .collect(),
            }),
            _ => None,
        };
        ready(Ok(record)).boxed()
    }

    fn insert_desktop_oauth_state(&self, _: DesktopOAuthState) -> AuthResult<'_, ()> {
        ready(Ok(())).boxed()
    }

    fn consume_desktop_oauth_state<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: DateTime,
    ) -> AuthResult<'a, Option<DesktopOAuthState>> {
        ready(Ok(None)).boxed()
    }

    fn delete_session_by_id<'a>(&'a self, sid: &'a str) -> AuthResult<'a, ()> {
        if sid == self.alice_sid.lock().expect("session cookie").as_str() {
            drop(self.alice.lock().expect("sessions").take());
        }
        ready(Ok(())).boxed()
    }

    fn list_active_sessions<'a>(
        &'a self,
        _: &'a str,
        _: DateTime,
    ) -> AuthResult<'a, Vec<ListedSessionRecord>> {
        ready(Ok(vec![])).boxed()
    }

    fn delete_user_session<'a>(&'a self, _: &'a str, _: ObjectId) -> AuthResult<'a, bool> {
        ready(Ok(false)).boxed()
    }

    fn delete_user_sessions<'a>(&'a self, _: &'a str) -> AuthResult<'a, ()> {
        ready(Ok(())).boxed()
    }

    fn delete_account<'a>(&'a self, _: &'a str) -> AuthResult<'a, ()> {
        ready(Ok(())).boxed()
    }

    fn insert_oauth_state(&self, value: OAuthStateRecord) -> AuthResult<'_, ()> {
        self.upstream.lock().expect("upstream").push(value);
        ready(Ok(())).boxed()
    }

    fn consume_oauth_state<'a>(
        &'a self,
        state: &'a str,
    ) -> AuthResult<'a, Option<OAuthStateRecord>> {
        let mut states = self.upstream.lock().expect("upstream");
        let record = states
            .iter()
            .position(|record| record.state == state)
            .map(|index| states.swap_remove(index));
        ready(Ok(record)).boxed()
    }

    fn upsert_discord_user<'a>(&'a self, _: DiscordUserUpsert) -> AuthResult<'a, ()> {
        ready(Ok(())).boxed()
    }

    fn insert_session<'a>(&'a self, record: NewSessionRecord) -> AuthResult<'a, ()> {
        *self.alice_sid.lock().expect("session cookie") = record.session_id;
        *self.alice.lock().expect("sessions") = Some(SessionRecord {
            id: ObjectId::new(),
            user_id: record.user_id,
            created_at: record.created_at,
            expires_at: record.expires_at,
        });
        ready(Ok(())).boxed()
    }
}

struct Memory {
    registrations: Mutex<FxHashMap<ObjectId, String>>,
    requests: Mutex<FxHashMap<String, Request>>,
    grants: Mutex<FxHashMap<String, Grant>>,
    codes: Mutex<FxHashMap<String, Code>>,
    families: Mutex<FxHashMap<ObjectId, RefreshFamily>>,
}

#[async_trait]
impl OidcRepository for Memory {
    async fn registration(&self, id: ObjectId) -> MongoResult<Option<DeveloperApp>> {
        let owner = self.registrations.lock().expect("registrations").get(&id).cloned();
        Ok(owner.map(|owner_id| DeveloperApp {
            id,
            owner_id,
            terms_accepted_at: DateTime::now(),
            settings: DeveloperAppSettings {
                name: "Example tool".to_owned(),
                description: None,
                terms_of_service_url: None,
                privacy_policy_url: None,
                redirect_uris: vec![CALLBACK.to_owned()],
            },
        }))
    }

    async fn subject(&self, user: &str) -> MongoResult<Option<String>> {
        Ok(Some(format!("subject-{user}")))
    }

    async fn insert_request(&self, request: &Request) -> MongoResult<()> {
        self.requests.lock().expect("requests").insert(request.id.clone(), request.clone());
        Ok(())
    }

    async fn request(
        &self,
        id: &str,
        browser: &str,
        now: DateTime,
    ) -> MongoResult<Option<Request>> {
        Ok(self
            .requests
            .lock()
            .expect("requests")
            .get(id)
            .filter(|request| request.browser_hash == browser && request.expires_at > now)
            .cloned())
    }

    async fn bind_request(
        &self,
        id: &str,
        browser: &str,
        binding: &BrowserBinding,
        now: DateTime,
    ) -> MongoResult<Option<Request>> {
        let mut requests = self.requests.lock().expect("requests");
        let Some(request) = requests
            .get_mut(id)
            .filter(|request| request.browser_hash == browser && request.expires_at > now)
        else {
            return Ok(None);
        };
        let session_id = binding.session_id;
        let bound = request.binding.get_or_insert_with(|| binding.clone());
        Ok((bound.session_id == session_id).then(|| request.clone()))
    }

    async fn cancel_unbound_request(
        &self,
        id: &str,
        browser: &str,
        now: DateTime,
    ) -> MongoResult<bool> {
        let mut requests = self.requests.lock().expect("requests");
        if requests.get(id).is_some_and(|request| {
            request.browser_hash == browser && request.expires_at > now && request.binding.is_none()
        }) {
            return Ok(requests.remove(id).is_some());
        }
        Ok(false)
    }

    async fn consume_request(
        &self,
        id: &str,
        browser: &str,
        session: ObjectId,
        now: DateTime,
    ) -> MongoResult<bool> {
        let mut requests = self.requests.lock().expect("requests");
        if requests.get(id).is_some_and(|request| {
            request.browser_hash == browser
                && request.expires_at > now
                && request.binding.as_ref().is_some_and(|binding| binding.session_id == session)
        }) {
            return Ok(requests.remove(id).is_some());
        }
        Ok(false)
    }

    async fn grant(&self, id: &str) -> MongoResult<Option<Grant>> {
        Ok(self.grants.lock().expect("grants").get(id).cloned())
    }

    async fn save_grant(&self, next: &Grant, previous: Option<ObjectId>) -> MongoResult<bool> {
        let mut grants = self.grants.lock().expect("grants");
        if grants.get(&next.id).map(|grant| grant.version) != previous {
            return Ok(false);
        }
        grants.insert(next.id.clone(), next.clone());
        Ok(true)
    }

    async fn list_grants(&self, user: &str) -> MongoResult<Vec<Grant>> {
        Ok(self
            .grants
            .lock()
            .expect("grants")
            .values()
            .filter(|grant| grant.user_id == user && !grant.revoked)
            .cloned()
            .collect())
    }

    async fn revoke_grant(&self, id: &str, user: &str) -> MongoResult<bool> {
        let mut grants = self.grants.lock().expect("grants");
        let Some(grant) =
            grants.get_mut(id).filter(|grant| grant.user_id == user && !grant.revoked)
        else {
            return Ok(false);
        };
        grant.revoked = true;
        grant.version = ObjectId::new();
        Ok(true)
    }

    async fn insert_code(&self, code: &Code) -> MongoResult<()> {
        self.codes.lock().expect("codes").insert(code.hash.clone(), code.clone());
        Ok(())
    }

    async fn code(&self, hash: &str, now: DateTime) -> MongoResult<Option<Code>> {
        Ok(self
            .codes
            .lock()
            .expect("codes")
            .get(hash)
            .filter(|code| code.expires_at > now)
            .cloned())
    }

    async fn consume_code(&self, hash: &str, now: DateTime) -> MongoResult<bool> {
        let mut codes = self.codes.lock().expect("codes");
        Ok(codes.get(hash).is_some_and(|code| code.expires_at > now)
            && codes.remove(hash).is_some())
    }

    async fn insert_family(&self, family: &RefreshFamily) -> MongoResult<()> {
        self.families.lock().expect("families").insert(family.id, family.clone());
        Ok(())
    }

    async fn family(&self, id: ObjectId) -> MongoResult<Option<RefreshFamily>> {
        Ok(self.families.lock().expect("families").get(&id).cloned())
    }

    async fn rotate_family(&self, next: &RefreshFamily, current: &str) -> MongoResult<bool> {
        let mut families = self.families.lock().expect("families");
        let Some(family) = families
            .get_mut(&next.id)
            .filter(|family| !family.revoked && family.current_hash == current)
        else {
            return Ok(false);
        };
        *family = next.clone();
        Ok(true)
    }

    async fn revoke_replayed_family(&self, id: ObjectId, spent: &str) -> MongoResult<()> {
        if let Some(family) = self.families.lock().expect("families").get_mut(&id)
            && family.spent_hashes.iter().any(|hash| hash == spent)
        {
            family.revoked = true;
        }
        Ok(())
    }
}

pub(super) struct Fixture {
    router: Router,
    auth: Arc<Auth>,
    memory: Arc<Memory>,
    pub(super) id: String,
}

impl Fixture {
    pub(super) async fn new() -> Self {
        let database = Client::with_uri_str("mongodb://127.0.0.1:1")
            .await
            .expect("construct client without connecting")
            .database("unused");
        let auth = Arc::new(Auth {
            alice: Mutex::new(Some(session(USER))),
            alice_sid: Mutex::new("alice".to_owned()),
            bob: session(BOB),
            upstream: Mutex::new(Vec::new()),
            owner_in_program: AtomicBool::new(true),
        });
        let app = ObjectId::new();
        let memory = Arc::new(Memory {
            registrations: Mutex::new([(app, APP_OWNER.to_owned())].into_iter().collect()),
            requests: Mutex::default(),
            grants: Mutex::default(),
            codes: Mutex::default(),
            families: Mutex::default(),
        });
        let seed = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        let config = OidcConfig::new(ORIGIN, &seed, "").expect("issuer config");
        let oidc = Oidc {
            config,
            store: memory.clone(),
            app_limiter: RateLimiter::new(RateLimitMode::Local, 120, Duration::from_millis(250)),
        };
        let mut state = AppState::new(
            auth.clone(),
            GameQueryStore::new(database.clone()),
            GameLocalizationStore::new(database.clone()),
            ReportsStore::new(database.clone()),
            TerritoryPlannerV2Store::new(database),
            DiscordOAuthConfig {
                client_id: "upstream".to_owned(),
                client_secret: "unused".to_owned(),
                redirect_uri: format!("{ORIGIN}/proxy/v1/auth/discord/callback"),
            },
            None,
        )
        .with_oidc(oidc);
        state.test_discord_identity = Some(DiscordUserUpsert {
            discord_id: USER.to_owned(),
            email: "upstream@example.test".to_owned(),
        });
        let router = crate::build_router(Arc::new(state), RateLimitMode::Local);
        Self { router, auth, memory, id: app.to_hex() }
    }

    pub(super) fn register_app(&self, id: ObjectId) {
        self.memory.registrations.lock().expect("registrations").insert(id, APP_OWNER.to_owned());
    }

    pub(super) fn remove_owner_from_developer_program(&self) {
        self.auth.owner_in_program.store(false, Ordering::Relaxed);
    }

    pub(super) fn expire_access_tokens(&self) {
        let mut families = self.memory.families.lock().expect("families");
        let family = families.values_mut().next().expect("family");
        for access in &mut family.access_tokens {
            access.expires_at = DateTime::from_millis(0);
        }
    }

    pub(super) fn authorization_url(&self, scopes: &str, redirect: &str) -> String {
        let mut url = Url::parse(&format!("{ORIGIN}/v1/auth/authorize")).expect("authorize URL");
        url.query_pairs_mut()
            .append_pair("client_id", &self.id)
            .append_pair("scope", scopes)
            .append_pair("redirect_uri", redirect)
            .append_pair("state", "client-state")
            .append_pair("nonce", "test-nonce")
            .append_pair("response_type", "code")
            .append_pair("code_challenge_method", "S256")
            .append_pair("code_challenge", &hash(VERIFIER));
        url.to_string()
    }

    pub(super) async fn call(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: String,
    ) -> Response {
        let mut request = HttpRequest::builder().method(method).uri(uri.replace("/proxy/", "/"));
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.router
            .clone()
            .oneshot(request.body(Body::from(body)).expect("request"))
            .await
            .expect("response")
    }

    pub(super) async fn start(&self, scopes: &str) -> (String, String) {
        let url = self.authorization_url(scopes, CALLBACK);
        let started = self.call(Method::GET, &url, &[("cookie", "sid=alice")], String::new()).await;
        assert_eq!(started.status(), StatusCode::SEE_OTHER);
        let cookie = started
            .headers()
            .get(header::SET_COOKIE)
            .expect("cookie")
            .to_str()
            .expect("cookie string")
            .split(';')
            .next()
            .expect("browser token");
        let cookie = format!("sid=alice; {cookie}");
        let resumed =
            self.call(Method::GET, location(&started), &[("cookie", &cookie)], String::new()).await;
        assert_eq!(resumed.status(), StatusCode::SEE_OTHER);
        (location(&resumed).to_owned(), cookie)
    }

    pub(super) async fn decide(
        &self,
        review: &str,
        cookie: &str,
        origin: &str,
        decision: &str,
    ) -> Response {
        let url = Url::parse(&format!("{ORIGIN}{review}")).expect("review URL");
        let request = query(&url, "request");
        let form = form_data(&[("request", &request), ("decision", decision)]);
        self.call(
            Method::POST,
            "/v1/auth/authorize/decision",
            &[
                ("cookie", cookie),
                ("origin", origin),
                ("content-type", "application/x-www-form-urlencoded"),
            ],
            form,
        )
        .await
    }

    pub(super) async fn code(&self, scopes: &str) -> String {
        let (review, cookie) = self.start(scopes).await;
        let response = self.decide(&review, &cookie, ORIGIN, "allow").await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let callback = Url::parse(location(&response)).expect("callback URL");
        assert_eq!(query(&callback, "state"), "client-state");
        assert_eq!(query(&callback, "source"), "fixture");
        query(&callback, "code")
    }

    pub(super) async fn exchange(&self, code: &str, verifier: &str) -> Response {
        self.token(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", CALLBACK),
            ("code_verifier", verifier),
        ])
        .await
    }

    pub(super) async fn token(&self, data: &[(&str, &str)]) -> Response {
        let mut data = data.to_vec();
        data.push(("client_id", &self.id));
        self.call(
            Method::POST,
            "/v1/auth/token",
            &[("content-type", "application/x-www-form-urlencoded")],
            form_data(&data),
        )
        .await
    }

    pub(super) async fn profile(&self, access: &str, cookie: &str) -> Response {
        self.call(
            Method::GET,
            "/v1/me/profile",
            &[
                ("authorization", &format!("Bearer {access}")),
                ("user-agent", &format!("ROKBattles/{} (Renamed)", self.id)),
                ("cookie", cookie),
            ],
            String::new(),
        )
        .await
    }
}

fn session(user: &str) -> SessionRecord {
    let now = DateTime::now();
    SessionRecord {
        id: ObjectId::new(),
        user_id: user.to_owned(),
        created_at: now,
        expires_at: after(now, 3600),
    }
}

pub(super) fn location(response: &Response) -> &str {
    response.headers().get(header::LOCATION).expect("redirect").to_str().expect("location")
}

pub(super) fn query(url: &Url, key: &str) -> String {
    url.query_pairs()
        .find_map(|(name, value)| (name == key).then(|| value.into_owned()))
        .expect(key)
}

fn form_data(values: &[(&str, &str)]) -> String {
    let mut url = Url::parse("https://example.test/").expect("url");
    url.query_pairs_mut().extend_pairs(values.iter().copied());
    url.query().expect("query").to_owned()
}

pub(super) async fn body(response: Response) -> Value {
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.expect("body");
    serde_json::from_slice(&bytes).expect("JSON")
}

pub(super) fn claim<'a>(json: &'a Value, key: &str) -> &'a str {
    json.get(key).and_then(Value::as_str).expect(key)
}
