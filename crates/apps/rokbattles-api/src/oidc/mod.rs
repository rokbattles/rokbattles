//! Private authorization server for approved Developer Program apps.

use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mongodb::bson::{DateTime, oid::ObjectId};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    auth::Role,
    db::{AuthRepository, DeveloperApp},
    error::ApiError,
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

mod config;
mod flow;
mod grants;
mod store;
#[cfg(test)]
mod test_support;
mod token;

use config::ALGORITHM;
pub use config::OidcConfig;
pub(crate) use flow::{browser_continuation, resume_path};
use grants::grant_routes;
pub(crate) use store::GRANTS_COLLECTION;
use store::{MongoOidcStore, OidcRepository};
pub(crate) use token::{AppAccess, authorize_access, bearer_rejection};

pub(crate) const AUTHORIZE_PATH: &str = "/proxy/v1/auth/authorize";
const TOKEN_PATH: &str = "/proxy/v1/auth/token";
const JWKS_PATH: &str = "/proxy/v1/auth/jwks";
const PROFILE_PATH: &str = "/proxy/v1/me/profile";

/// Runtime dependencies of the OIDC provider.
pub struct Oidc {
    config: OidcConfig,
    store: Arc<dyn OidcRepository>,
    pub(crate) app_limiter: RateLimiter,
}

impl Oidc {
    /// Prepares storage for the issuer.
    pub async fn new(
        config: OidcConfig,
        db: mongodb::Database,
        mode: RateLimitMode,
    ) -> mongodb::error::Result<Self> {
        let store = MongoOidcStore::new(db);
        store.ensure_indexes().await?;

        Ok(Self {
            config,
            store: Arc::new(store),
            app_limiter: RateLimiter::new(mode, 120, Duration::from_millis(250)),
        })
    }

    pub(crate) fn authorize_url(&self) -> String {
        format!("{}{AUTHORIZE_PATH}", self.config.issuer)
    }

    async fn eligible_app(
        &self,
        accounts: &dyn AuthRepository,
        client_id: ObjectId,
    ) -> Result<Option<DeveloperApp>, ApiError> {
        let Some(app) = self.store.registration(client_id).await.map_err(storage_error)? else {
            return Ok(None);
        };
        let owner = accounts.find_user_by_discord_id(&app.owner_id).await.map_err(account_error)?;
        Ok(owner.is_some_and(|owner| owner.roles.contains(Role::DeveloperProgram)).then_some(app))
    }

    pub(crate) fn origin(&self) -> &str {
        &self.config.issuer
    }
}

/// A permission that can be requested by a registered application.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Scope {
    #[serde(rename = "openid")]
    OpenId,
    #[serde(rename = "profile:read")]
    ProfileRead,
    #[serde(rename = "governor.binds:read")]
    GovernorBindsRead,
    #[serde(rename = "governor.binds:write")]
    GovernorBindsWrite,
    #[serde(rename = "governor.data:read")]
    GovernorDataRead,
}

#[derive(Clone, Serialize)]
pub(crate) struct ScopeInfo {
    id: Scope,
    description: &'static str,
}

impl Scope {
    pub(crate) const ALL: [Self; 5] = [
        Self::OpenId,
        Self::ProfileRead,
        Self::GovernorBindsRead,
        Self::GovernorBindsWrite,
        Self::GovernorDataRead,
    ];

    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::OpenId => "openid",
            Self::ProfileRead => "profile:read",
            Self::GovernorBindsRead => "governor.binds:read",
            Self::GovernorBindsWrite => "governor.binds:write",
            Self::GovernorDataRead => "governor.data:read",
        }
    }

    fn is_governor_scope(self) -> bool {
        matches!(self, Self::GovernorBindsRead | Self::GovernorBindsWrite | Self::GovernorDataRead)
    }

    pub(crate) fn info(self) -> ScopeInfo {
        let description = match self {
            Self::OpenId => "Sign in with ROK Battles",
            Self::ProfileRead => "View your linked Discord ID",
            Self::GovernorBindsRead => "View your governor binds",
            Self::GovernorBindsWrite => {
                "Add, remove, and change the default for your governor binds"
            }
            Self::GovernorDataRead => {
                "View battle reports, loot, and resources for your primary binds and governors currently shared with you, including primary binds added later"
            }
        };

        ScopeInfo { id: self, description }
    }

    fn parse(scopes: &str) -> Result<Vec<Self>, &'static str> {
        let mut parsed = scopes
            .split(' ')
            .map(|id| Self::ALL.into_iter().find(|scope| scope.id() == id).ok_or("invalid_scope"))
            .collect::<Result<Vec<_>, _>>()?;
        parsed.sort_unstable();
        parsed.dedup();
        Ok(parsed)
    }
}

fn scope_string(scopes: &[Scope]) -> String {
    scopes.iter().map(|scope| scope.id()).collect::<Vec<_>>().join(" ")
}

pub(super) fn hash(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(value.as_bytes()))
}

pub(super) fn random_token() -> String {
    let mut bytes = [0; 32];
    rand::rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn opaque_token(value: &str) -> bool {
    value.len() == 43
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

fn after(now: DateTime, seconds: i64) -> DateTime {
    DateTime::from_millis(now.timestamp_millis().saturating_add(seconds * 1000))
}

pub(crate) fn router(mode: RateLimitMode) -> Router<Arc<AppState>> {
    let browser = Arc::new(RateLimiter::new(mode, 20, Duration::from_secs(4)));
    let tokens = Arc::new(RateLimiter::new(mode, 30, Duration::from_secs(2)));

    Router::new()
        .route("/authorize", get(flow::authorize))
        .route("/authorize/resume", get(flow::resume))
        .route("/authorize/consent", get(flow::consent))
        .route("/authorize/consent/email", get(flow::consent_email))
        .route("/authorize/decision", post(flow::decision))
        .route_layer(middleware::from_fn_with_state(browser, rate_limit::enforce))
        .merge(
            Router::new()
                .route("/token", post(token::exchange))
                .route_layer(middleware::from_fn_with_state(tokens, rate_limit::enforce)),
        )
        .merge(grant_routes(mode))
        .route("/jwks", get(jwks))
        .layer(middleware::from_fn(private_response))
}

pub(crate) async fn discovery(State(state): State<Arc<AppState>>) -> Response {
    let Ok(oidc) = provider(&state) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let issuer = &oidc.config.issuer;
    let response = Json(serde_json::json!({
        "issuer": issuer,
        "authorization_endpoint": oidc.authorize_url(),
        "token_endpoint": format!("{issuer}{TOKEN_PATH}"),
        "jwks_uri": format!("{issuer}{JWKS_PATH}"),
        "userinfo_endpoint": format!("{issuer}{PROFILE_PATH}"),
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["none"],
        "code_challenge_methods_supported": ["S256"],
        "subject_types_supported": ["public"],
        "scopes_supported": Scope::ALL,
        "claims_supported": ["iss", "sub", "aud", "iat", "exp", "nonce", "discord_id"],
        "id_token_signing_alg_values_supported": [ALGORITHM],
    }));

    ([(header::CACHE_CONTROL, "public, max-age=60")], response).into_response()
}

async fn jwks(State(state): State<Arc<AppState>>) -> Result<Response, ApiError> {
    let oidc = provider(&state)?;
    Ok(([(header::CACHE_CONTROL, "public, max-age=60")], Json(oidc.config.jwks())).into_response())
}

pub(crate) async fn account_subject(state: &AppState, user_id: &str) -> Result<String, ApiError> {
    provider(state)?
        .store
        .subject(user_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(ApiError::unauthorized)
}

fn provider(state: &AppState) -> Result<&Oidc, ApiError> {
    state.oidc.as_deref().ok_or_else(|| ApiError::not_found("OIDC is unavailable"))
}

pub(crate) async fn private_response(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    set_private_headers(response.headers_mut());
    response
}

pub(crate) fn set_private_headers(headers: &mut HeaderMap) {
    headers.entry(header::CACHE_CONTROL).or_insert(HeaderValue::from_static("no-store"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
}

pub(crate) fn same_origin(headers: &HeaderMap, expected: &str) -> Result<(), ApiError> {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    if origins.next().and_then(|origin| origin.to_str().ok()) != Some(expected)
        || origins.next().is_some()
    {
        return Err(ApiError::Forbidden("invalid-origin".to_owned()));
    }
    Ok(())
}

fn storage_error(error: mongodb::error::Error) -> ApiError {
    // Database errors can include command data, such as token hashes or callback URLs.
    tracing::error!(kind = ?std::mem::discriminant(&*error.kind), "OIDC storage operation failed");
    ApiError::internal("OIDC storage operation failed")
}

fn account_error(error: crate::db::AuthStoreError) -> ApiError {
    match error {
        crate::db::AuthStoreError::Database(error) => storage_error(error),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, StatusCode, header};
    use mongodb::bson::oid::ObjectId;
    use serde_json::json;

    use super::{
        Scope, scope_string,
        test_support::{CALLBACK, Fixture, VERIFIER, body, claim},
    };

    #[tokio::test]
    async fn a_new_registration_can_request_every_supported_scope_without_restarting() {
        let mut fixture = Fixture::new().await;
        let id = ObjectId::new();
        fixture.id = id.to_hex();
        let scopes = scope_string(&Scope::ALL);
        let url = fixture.authorization_url(&scopes, CALLBACK);
        let unregistered =
            fixture.call(Method::GET, &url, &[("cookie", "sid=alice")], String::new()).await;
        assert_eq!(unregistered.status(), StatusCode::BAD_REQUEST);
        assert!(!unregistered.headers().contains_key(header::LOCATION));

        fixture.register_app(id);
        let code = fixture.code(&scopes).await;
        let issued = fixture.exchange(&code, VERIFIER).await;
        assert_eq!(issued.status(), StatusCode::OK);
        assert_eq!(body(issued).await["scope"], scopes);
    }

    #[tokio::test]
    async fn delegated_access_requires_the_owner_to_remain_in_the_developer_program() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
        let access = claim(&issued, "access_token");
        let refresh = claim(&issued, "refresh_token");
        assert_eq!(fixture.profile(access, "").await.status(), StatusCode::OK);

        fixture.remove_owner_from_developer_program();

        assert_eq!(fixture.profile(access, "").await.status(), StatusCode::UNAUTHORIZED);
        let rotated =
            fixture.token(&[("grant_type", "refresh_token"), ("refresh_token", refresh)]).await;
        assert_eq!(rotated.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(rotated).await, json!({ "error": "invalid_grant" }));

        let url = fixture.authorization_url("openid", CALLBACK);
        let refused =
            fixture.call(Method::GET, &url, &[("cookie", "sid=alice")], String::new()).await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert!(!refused.headers().contains_key(header::LOCATION));
    }
}
