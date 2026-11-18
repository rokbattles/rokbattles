//! Account Settings endpoints for listing and deauthorizing connected apps.

use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{delete, get},
};
use mongodb::bson::oid::ObjectId;
use serde::Serialize;

use super::{Scope, ScopeInfo, provider, same_origin, storage_error, store::grant_id};
use crate::{
    auth::AuthenticatedSession,
    error::ApiError,
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizedApp {
    id: String,
    name: String,
    authorized_on: i64,
    terms_url: Option<String>,
    privacy_url: Option<String>,
    permissions: Vec<ScopeInfo>,
}

pub(super) fn grant_routes(mode: RateLimitMode) -> Router<Arc<AppState>> {
    let limiter = Arc::new(RateLimiter::new(mode, 12, Duration::from_secs(5)));

    Router::new()
        .route("/authorized-apps", get(list))
        .route("/authorized-apps/{app_id}", delete(revoke))
        .layer(middleware::from_fn_with_state(limiter, rate_limit::enforce))
}

async fn list(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let Some(oidc) = &state.oidc else {
        return Ok(Json(Vec::new()));
    };
    let grants = oidc.store.list_grants(&session.user.discord_id).await.map_err(storage_error)?;
    let mut apps = Vec::with_capacity(grants.len());

    for grant in grants {
        let Some(app) = oidc.store.registration(grant.client_id).await.map_err(storage_error)?
        else {
            continue;
        };
        if grant.owner_id != app.owner_id {
            continue;
        }

        apps.push(AuthorizedApp {
            id: app.id.to_hex(),
            name: app.settings.name,
            authorized_on: grant.authorized_on.timestamp_millis(),
            terms_url: app.settings.terms_of_service_url,
            privacy_url: app.settings.privacy_policy_url,
            permissions: grant.scopes.into_iter().map(Scope::info).collect(),
        });
    }

    Ok(Json(apps))
}

async fn revoke(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(app_id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let oidc = provider(&state)?;
    same_origin(&headers, oidc.origin())?;
    let app_id =
        ObjectId::parse_str(app_id).map_err(|_error| ApiError::not_found("app not found"))?;

    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;

    let subject = oidc
        .store
        .subject(&session.user.discord_id)
        .await
        .map_err(storage_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let id = grant_id(&subject, app_id);
    oidc.store.revoke_grant(&id, &session.user.discord_id).await.map_err(storage_error)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, StatusCode};

    use crate::oidc::test_support::{Fixture, ORIGIN, VERIFIER, body, claim};

    #[tokio::test]
    async fn deauthorization_invalidates_pending_codes_and_tokens() {
        let fixture = Fixture::new().await;
        let code = fixture.code("openid profile:read").await;
        let issued = body(fixture.exchange(&code, VERIFIER).await).await;
        let list = fixture
            .call(
                Method::GET,
                "/v1/auth/authorized-apps",
                &[("cookie", "sid=alice")],
                String::new(),
            )
            .await;
        let list = body(list).await;
        assert_eq!(
            list.as_array().and_then(|apps| apps.first()).expect("connected app")["id"],
            fixture.id
        );
        let stale_code = fixture.code("openid").await;
        let revoked = fixture
            .call(
                Method::DELETE,
                &format!("/v1/auth/authorized-apps/{}", fixture.id),
                &[("cookie", "sid=alice"), ("origin", ORIGIN)],
                String::new(),
            )
            .await;
        assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            fixture.profile(claim(&issued, "access_token"), "sid=alice").await.status(),
            StatusCode::UNAUTHORIZED
        );
        let refresh = fixture
            .token(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", claim(&issued, "refresh_token")),
            ])
            .await;
        assert_eq!(refresh.status(), StatusCode::BAD_REQUEST);
        assert_eq!(fixture.exchange(&stale_code, VERIFIER).await.status(), StatusCode::BAD_REQUEST);
    }
}
