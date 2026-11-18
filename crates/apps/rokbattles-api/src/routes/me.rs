//! Shared account routes. Only routes marked with a scope accept developer tokens.

use std::{sync::Arc, time::Duration};

use axum::{
    Extension, Json, Router,
    extract::{FromRequestParts, State},
    http::{header, request::Parts},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use reqwest::Url;

use super::{
    auth::load_claimed_governors,
    governor::{bind, loot, resources},
    reports::battle,
};
use crate::{
    auth::AuthenticatedSession,
    error::ApiError,
    oidc::{self, AppAccess, Scope},
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

pub(super) fn router(mode: RateLimitMode) -> Router<Arc<AppState>> {
    let reads = Router::new()
        .route("/profile", get(profile).post(profile))
        .route_layer(Extension(Scope::OpenId))
        .merge(
            Router::new()
                .route("/governor-binds", get(list_binds))
                .route_layer(Extension(Scope::GovernorBindsRead)),
        )
        .merge(
            Router::new()
                .route("/governors/{governor_id}/reports/battle", get(battle::get_mine))
                .route(
                    "/governors/{governor_id}/reports/battle/{report_id}",
                    get(battle::get_my_report),
                )
                .route("/governors/{governor_id}/loot/barbarians", get(loot::get_barbarians))
                .route(
                    "/governors/{governor_id}/loot/barbarian-forts",
                    get(loot::get_barbarian_forts),
                )
                .route("/governors/{governor_id}/loot/baulurs", get(loot::get_baulurs))
                .route("/governors/{governor_id}/loot/lohars-trial", get(loot::get_lohars_trial))
                .route(
                    "/governors/{governor_id}/loot/karuak-ceremony",
                    get(loot::get_karuak_ceremony),
                )
                .route(
                    "/governors/{governor_id}/loot/kahars-treasure",
                    get(loot::get_kahars_treasure),
                )
                .route("/governors/{governor_id}/resources", get(resources::get))
                .route_layer(Extension(Scope::GovernorDataRead)),
        );

    let writes = Router::new()
        .route(
            "/governor-binds/{governor_id}",
            axum::routing::post(bind::post).delete(bind::delete),
        )
        .route("/governor-binds/{governor_id}/default", axum::routing::patch(bind::patch_default))
        .route_layer(Extension(Scope::GovernorBindsWrite))
        .route_layer(middleware::from_fn_with_state(
            Arc::new(RateLimiter::new(mode, 10, Duration::from_secs(4))),
            rate_limit::enforce,
        ));

    reads.merge(writes).layer(middleware::from_fn(oidc::private_response))
}

/// The account making a request. Legacy routes use the browser session branch.
pub(crate) enum AccountAccess {
    Session(AuthenticatedSession),
    App(AppAccess),
}

impl AccountAccess {
    pub(crate) fn account_id(&self) -> &str {
        match self {
            Self::Session(session) => &session.user.discord_id,
            Self::App(access) => &access.account_id,
        }
    }

    pub(crate) async fn ensure_active(&self, state: &AppState) -> Result<(), ApiError> {
        match self {
            Self::Session(session) => session.ensure_active(state.auth_store.as_ref()).await,
            Self::App(access) => access.ensure_active(state).await,
        }
    }
}

impl FromRequestParts<Arc<AppState>> for AccountAccess {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let scope = parts.extensions.get::<Scope>().copied();
        if let Some(scope) = scope
            && parts.headers.contains_key(header::AUTHORIZATION)
        {
            let access = oidc::authorize_access(state, &parts.headers)
                .await
                .map_err(oidc::bearer_rejection)?;
            if !access.scopes.contains(&scope) {
                return Err(oidc::bearer_rejection(ApiError::Forbidden(
                    "insufficient_scope".to_owned(),
                )));
            }
            if let Some(oidc) = &state.oidc {
                oidc.app_limiter
                    .check_app(access.client_id)
                    .map_err(IntoResponse::into_response)?;
            }
            return Ok(Self::App(access));
        }

        let session =
            AuthenticatedSession::from_request_parts(parts, state).await.map_err(|error| {
                if scope.is_some() && matches!(error, ApiError::Unauthorized) {
                    return ([(header::WWW_AUTHENTICATE, "Bearer")], error).into_response();
                }
                error.into_response()
            })?;
        if scope == Some(Scope::GovernorBindsWrite) {
            let origin = match &state.oidc {
                Some(oidc) => oidc.origin().to_owned(),
                None => Url::parse(&state.discord_oauth.redirect_uri)
                    .ok()
                    .map(|url| url.origin().ascii_serialization())
                    .filter(|origin| origin != "null")
                    .ok_or_else(|| ApiError::internal("invalid site origin").into_response())?,
            };
            oidc::same_origin(&parts.headers, &origin).map_err(IntoResponse::into_response)?;
        }
        Ok(Self::Session(session))
    }
}

async fn profile(
    State(state): State<Arc<AppState>>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    #[derive(serde::Serialize)]
    struct Profile {
        sub: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        discord_id: Option<String>,
    }

    let profile = match access {
        AccountAccess::Session(session) => Profile {
            sub: oidc::account_subject(&state, &session.user.discord_id).await?,
            discord_id: Some(session.user.discord_id),
        },
        AccountAccess::App(access) => Profile {
            sub: access.subject,
            discord_id: access.scopes.contains(&Scope::ProfileRead).then_some(access.account_id),
        },
    };
    Ok(Json(profile))
}

async fn list_binds(
    State(state): State<Arc<AppState>>,
    access: AccountAccess,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(load_claimed_governors(&state, access.account_id()).await?))
}
