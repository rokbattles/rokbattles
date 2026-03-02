use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use futures::StreamExt;
use mongodb::bson::{Bson, DateTime, Document, doc};
use mongodb::options::FindOptions;
use reqwest::Url;
use serde::Deserialize;

use crate::auth::{AuthenticatedSession, extract_cookie_value};
use crate::bson_utils::bson_to_i64_exact;
use crate::db::{DiscordUserUpsert, NewSessionRecord, OAuthStateRecord};
use crate::error::ApiError;
use crate::state::{AppState, DiscordOAuthConfig};

mod cookies;
mod oauth;
mod types;

use self::types::{AuthMeResponse, ClaimedGovernor, CurrentUser, LogoutResponse};

const OAUTH_STATE_TTL_MILLIS: i64 = 10 * 60 * 1000;
const SESSION_TTL_MILLIS: i64 = 7 * 24 * 60 * 60 * 1000;
const SESSION_MAX_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(get_me))
        .route("/logout", post(post_logout))
        .route("/discord/login", get(get_discord_login))
        .route("/discord/callback", get(get_discord_callback))
}

async fn get_me(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<AuthMeResponse>, ApiError> {
    let claimed_governors = load_claimed_governors(&state, &session.user.discord_id).await?;

    Ok(Json(AuthMeResponse {
        user: CurrentUser {
            discord_id: session.user.discord_id,
            email: session.user.email,
            claimed_governors,
        },
    }))
}

async fn post_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(sid) = extract_cookie_value(
        headers.get(COOKIE).and_then(|header| header.to_str().ok()),
        "sid",
    ) {
        state
            .auth_store
            .delete_session_by_id(&sid)
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;
    }

    let mut response = Json(LogoutResponse { success: true }).into_response();
    let clear_cookie = cookies::build_clear_session_cookie();
    response.headers_mut().append(
        SET_COOKIE,
        clear_cookie
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid cookie header: {error}")))?,
    );

    Ok(response)
}

async fn get_discord_login(State(state): State<Arc<AppState>>) -> Result<Redirect, ApiError> {
    let verifier = oauth::generate_code_verifier();
    let challenge = oauth::derive_code_challenge(&verifier);
    let state_token = oauth::generate_state();
    let now = DateTime::now();
    let expires_at = DateTime::from_millis(now.timestamp_millis() + OAUTH_STATE_TTL_MILLIS);

    state
        .auth_store
        .insert_oauth_state(OAuthStateRecord {
            state: state_token.clone(),
            verifier,
            created_at: now,
            expires_at,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut redirect_url = Url::parse("https://discord.com/oauth2/authorize")
        .map_err(|error| ApiError::internal(format!("invalid discord authorize url: {error}")))?;
    redirect_url
        .query_pairs_mut()
        .append_pair("client_id", &state.discord_oauth.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &state.discord_oauth.redirect_uri)
        .append_pair("scope", "identify email")
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state_token);

    Ok(Redirect::temporary(redirect_url.as_ref()))
}

async fn get_discord_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DiscordCallbackQuery>,
) -> Result<Response, ApiError> {
    if let Some(error) = query.error {
        return Err(ApiError::bad_request(error));
    }

    let Some(code) = query.code else {
        return Err(ApiError::bad_request("missing code"));
    };
    let Some(state_token) = query.state else {
        return Err(ApiError::bad_request("missing state"));
    };

    let Some(oauth_state) = state
        .auth_store
        .consume_oauth_state(&state_token)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
    else {
        return Err(ApiError::bad_request("invalid or expired state"));
    };

    if oauth_state.expires_at.timestamp_millis() <= DateTime::now().timestamp_millis() {
        return Err(ApiError::bad_request("invalid or expired state"));
    }

    let token = exchange_discord_token(&state.discord_oauth, &code, &oauth_state.verifier).await?;
    let profile = fetch_discord_profile(&token.access_token).await?;

    let Some(email) = profile.email else {
        return Err(ApiError::bad_request("discord email not verified"));
    };
    if !profile.verified.unwrap_or(false) {
        return Err(ApiError::bad_request("discord email not verified"));
    }

    state
        .auth_store
        .upsert_discord_user(DiscordUserUpsert {
            discord_id: profile.id.clone(),
            email,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let now = DateTime::now();
    let expires_at = DateTime::from_millis(now.timestamp_millis() + SESSION_TTL_MILLIS);
    let sid = oauth::generate_session_id();
    state
        .auth_store
        .insert_session(NewSessionRecord {
            session_id: sid.clone(),
            user_id: profile.id,
            created_at: now,
            expires_at,
        })
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut response = Redirect::to("/").into_response();
    let set_cookie = cookies::build_set_session_cookie(&sid, SESSION_MAX_AGE_SECONDS);
    response.headers_mut().append(
        SET_COOKIE,
        set_cookie
            .parse()
            .map_err(|error| ApiError::internal(format!("invalid cookie header: {error}")))?,
    );

    Ok(response)
}

async fn load_claimed_governors(
    state: &Arc<AppState>,
    discord_id: &str,
) -> Result<Vec<ClaimedGovernor>, ApiError> {
    let options = FindOptions::builder()
        .projection(doc! {
            "_id": 0,
            "governorId": 1,
            "governorName": 1,
            "governorAvatar": 1,
            "createdAt": 1,
            "default": 1
        })
        .build();

    let claims_collection = state.reports_store.claimed_governors_collection();
    let mut cursor = claims_collection
        .find(doc! { "discordId": discord_id })
        .with_options(options)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    let mut sortable_claims = Vec::new();
    while let Some(next) = cursor.next().await {
        let document = next.map_err(|error| ApiError::internal(error.to_string()))?;
        let Some(governor_id) = document.get("governorId").and_then(bson_to_i64_exact) else {
            continue;
        };

        sortable_claims.push(SortableClaim {
            created_at_millis: claim_created_at_millis(&document),
            claim: ClaimedGovernor {
                governor_id,
                governor_name: optional_string_field(&document, "governorName"),
                governor_avatar: optional_string_field(&document, "governorAvatar"),
                default: document.get_bool("default").unwrap_or(false),
            },
        });
    }

    sortable_claims.sort_by(|left, right| {
        right
            .claim
            .default
            .cmp(&left.claim.default)
            .then_with(|| right.created_at_millis.cmp(&left.created_at_millis))
    });

    Ok(sortable_claims
        .into_iter()
        .map(|entry| entry.claim)
        .collect())
}

fn optional_string_field(document: &Document, key: &str) -> Option<String> {
    document
        .get(key)
        .and_then(Bson::as_str)
        .map(ToString::to_string)
}

fn claim_created_at_millis(claim: &Document) -> i64 {
    match claim.get("createdAt") {
        Some(Bson::DateTime(value)) => value.timestamp_millis(),
        Some(value) => bson_to_i64_exact(value).unwrap_or(0),
        None => 0,
    }
}

async fn exchange_discord_token(
    oauth: &DiscordOAuthConfig,
    code: &str,
    verifier: &str,
) -> Result<DiscordTokenResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://discord.com/api/oauth2/token")
        .basic_auth(&oauth.client_id, Some(&oauth.client_secret))
        .form(&[
            ("client_id", oauth.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", oauth.redirect_uri.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if !response.status().is_success() {
        let reason = response
            .text()
            .await
            .unwrap_or_else(|_| "token exchange failed".to_string());
        return Err(ApiError::bad_request(reason));
    }

    response
        .json::<DiscordTokenResponse>()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

async fn fetch_discord_profile(access_token: &str) -> Result<DiscordProfileResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if !response.status().is_success() {
        let reason = response
            .text()
            .await
            .unwrap_or_else(|_| "failed to fetch profile".to_string());
        return Err(ApiError::bad_request(reason));
    }

    response
        .json::<DiscordProfileResponse>()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))
}

#[derive(Debug)]
struct SortableClaim {
    created_at_millis: i64,
    claim: ClaimedGovernor,
}

#[derive(Debug, Deserialize)]
struct DiscordCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct DiscordProfileResponse {
    id: String,
    email: Option<String>,
    verified: Option<bool>,
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{DateTime, doc};

    use super::*;

    #[test]
    fn extracts_optional_string_fields() {
        let document = doc! {
            "governorName": "test",
            "governorAvatar": mongodb::bson::Bson::Null
        };

        assert_eq!(
            optional_string_field(&document, "governorName"),
            Some("test".to_string())
        );
        assert_eq!(optional_string_field(&document, "governorAvatar"), None);
        assert_eq!(optional_string_field(&document, "missing"), None);
    }

    #[test]
    fn reads_claim_created_at_millis_from_datetime() {
        let created_at = DateTime::from_millis(1234);
        let document = doc! {
            "createdAt": created_at
        };

        assert_eq!(claim_created_at_millis(&document), 1234);
    }
}
