use std::time::{Duration, SystemTime};

use anyhow::{Context, anyhow};
use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use blake3::hash;
use mongodb::bson::{Bson, DateTime, Document, doc};
use rand::{Rng, distributions::Alphanumeric};
use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::AppState;

const DISCORD_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const DISCORD_USER_URL: &str = "https://discord.com/api/users/@me";
const SESSION_TTL: Duration = Duration::from_secs(60 * 60 * 24 * 7);

#[derive(Debug, Deserialize)]
pub struct DiscordCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: Option<String>,
    global_name: Option<String>,
    email: Option<String>,
    verified: Option<bool>,
    avatar: Option<String>,
}

pub async fn callback(
    State(st): State<AppState>,
    Query(query): Query<DiscordCallbackQuery>,
) -> Response {
    let redirect_base = determine_redirect(&st.auth_redirect_url, query.state.as_deref());

    if let Some(error_code) = query.error.as_deref() {
        warn!(error = error_code, "discord oauth returned error");
        return redirect_response(append_error(&redirect_base, "discord_denied"), None);
    }

    let code = match query.code.as_ref() {
        Some(code) => code.to_string(),
        None => {
            warn!("discord oauth callback missing code");
            return redirect_response(append_error(&redirect_base, "missing_code"), None);
        }
    };

    let token = match exchange_code(&st, &code).await {
        Ok(token) => token,
        Err(err) => {
            error!(error = ?err, "failed to exchange discord code");
            return redirect_response(append_error(&redirect_base, "token_exchange_failed"), None);
        }
    };

    let user = match fetch_user(&st, &token.access_token).await {
        Ok(user) => user,
        Err(err) => {
            error!(error = ?err, "failed to fetch discord user");
            return redirect_response(append_error(&redirect_base, "user_fetch_failed"), None);
        }
    };

    let now = SystemTime::now();
    if let Err(err) = persist_user(&st, &user, now).await {
        error!(error = ?err, user_id = %user.id, "failed to persist user");
        return redirect_response(append_error(&redirect_base, "user_persist_failed"), None);
    }

    let session_id = generate_session_id(48);
    let session_hash = hash(session_id.as_bytes()).to_hex().to_string();

    if let Err(err) = persist_session(&st, &user.id, &session_hash, &token, now).await {
        error!(error = ?err, user_id = %user.id, "failed to persist session");
        return redirect_response(append_error(&redirect_base, "session_create_failed"), None);
    }

    let cookie = build_session_cookie(&st, &session_id);
    let response = redirect_response(redirect_base, cookie);
    debug!(user_id = %user.id, "issued session cookie");
    response
}

async fn exchange_code(st: &AppState, code: &str) -> anyhow::Result<DiscordTokenResponse> {
    let response = st
        .http_client
        .post(DISCORD_TOKEN_URL)
        .form(&[
            ("client_id", st.discord_client_id.as_str()),
            ("client_secret", st.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", st.discord_redirect_uri.as_str()),
        ])
        .send()
        .await
        .context("failed to send discord token request")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "discord token exchange failed: status={} body={}",
            status,
            body
        ));
    }

    response
        .json::<DiscordTokenResponse>()
        .await
        .context("failed to decode discord token response")
}

async fn fetch_user(st: &AppState, access_token: &str) -> anyhow::Result<DiscordUser> {
    let response = st
        .http_client
        .get(DISCORD_USER_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("failed to send discord user request")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "discord user fetch failed: status={} body={}",
            status,
            body
        ));
    }

    response
        .json::<DiscordUser>()
        .await
        .context("failed to decode discord user response")
}

async fn persist_user(st: &AppState, user: &DiscordUser, now: SystemTime) -> anyhow::Result<()> {
    let users = st.db.collection::<Document>("users");
    let now_dt = DateTime::from_system_time(now);

    let filter = doc! { "_id": &user.id };

    let mut set = doc! {
        "discord.id": &user.id,
        "updatedAt": now_dt,
        "emailVerified": user.verified.unwrap_or(false),
    };

    set.insert("discord.username", string_or_null(user.username.as_deref()));
    set.insert(
        "discord.globalName",
        string_or_null(user.global_name.as_deref()),
    );
    set.insert("discord.avatar", string_or_null(user.avatar.as_deref()));
    set.insert("email", string_or_null(user.email.as_deref()));

    let update = doc! {
        "$set": set,
        "$setOnInsert": doc! {
            "createdAt": now_dt,
        },
    };

    users
        .update_one(filter, update)
        .upsert(true)
        .await
        .context("failed to upsert user")?;

    Ok(())
}

async fn persist_session(
    st: &AppState,
    user_id: &str,
    session_hash: &str,
    token: &DiscordTokenResponse,
    now: SystemTime,
) -> anyhow::Result<()> {
    let sessions = st.db.collection::<Document>("sessions");
    let created_at = DateTime::from_system_time(now);
    let expires_at = DateTime::from_system_time(now + SESSION_TTL);
    let token_expires_at = DateTime::from_system_time(now + Duration::from_secs(token.expires_in));

    let mut token_doc = doc! {
        "accessToken": &token.access_token,
        "tokenType": &token.token_type,
        "scope": &token.scope,
        "expiresAt": token_expires_at,
    };
    if let Some(refresh) = token.refresh_token.as_ref() {
        token_doc.insert("refreshToken", refresh);
    }

    let session_doc = doc! {
        "userId": user_id,
        "sessionIdHash": session_hash,
        "token": token_doc,
        "createdAt": created_at,
        "updatedAt": created_at,
        "expiresAt": expires_at,
        "lastUsedAt": created_at,
    };

    sessions
        .insert_one(session_doc)
        .await
        .context("failed to insert session")?;

    Ok(())
}

fn generate_session_id(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(len)
        .collect()
}

fn build_session_cookie(st: &AppState, session_id: &str) -> Option<String> {
    let max_age = SESSION_TTL.as_secs();
    if max_age == 0 {
        return None;
    }

    let mut cookie = format!("{}={}", st.session_cookie_name, session_id);
    cookie.push_str("; Path=/");
    cookie.push_str("; HttpOnly");
    cookie.push_str("; SameSite=Lax");
    if st.session_cookie_secure {
        cookie.push_str("; Secure");
    }
    if let Some(domain) = &st.session_cookie_domain {
        cookie.push_str("; Domain=");
        cookie.push_str(domain);
    }
    cookie.push_str(&format!("; Max-Age={}", max_age));

    Some(cookie)
}

fn determine_redirect(base: &str, state: Option<&str>) -> String {
    match state.and_then(sanitize_state) {
        Some(redirect_path) => {
            if is_absolute_url(base) {
                format!("{}{}", base.trim_end_matches('/'), redirect_path)
            } else {
                redirect_path
            }
        }
        None => base.to_string(),
    }
}

fn sanitize_state(state: &str) -> Option<String> {
    if state.is_empty() || !state.starts_with('/') || state.starts_with("//") {
        return None;
    }
    Some(state.to_string())
}

fn is_absolute_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn append_error(base: &str, code: &str) -> String {
    let mut url = base.to_string();
    if url.contains('?') {
        match url.chars().last() {
            Some('?') | Some('&') => {}
            _ => url.push('&'),
        }
    } else {
        url.push('?');
    }
    url.push_str("error=");
    url.push_str(code);
    url
}

fn redirect_response(location: String, cookie: Option<String>) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::FOUND;

    match HeaderValue::from_str(&location) {
        Ok(value) => {
            response.headers_mut().insert(header::LOCATION, value);
        }
        Err(err) => {
            error!(error = ?err, "invalid redirect location, falling back to '/'");
            response
                .headers_mut()
                .insert(header::LOCATION, HeaderValue::from_static("/"));
        }
    }

    if let Some(cookie) = cookie {
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(header::SET_COOKIE, value);
        } else {
            error!("failed to encode session cookie header");
        }
    }

    response
}

fn string_or_null(value: Option<&str>) -> Bson {
    match value {
        Some(v) => Bson::String(v.to_string()),
        None => Bson::Null,
    }
}
