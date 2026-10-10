use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mongodb::bson::{DateTime, Document, doc};
use rand::RngExt;
use rokbattles_bson::nested_i64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{auth::AuthenticatedSession, error::ApiError, state::AppState};

mod aggregate;
mod store;

const NO_STORE: [(&str, &str); 2] =
    [("Cache-Control", "no-store"), ("Referrer-Policy", "no-referrer")];
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const BATTLE_LIFETIME_MS: i64 = 5 * 60 * 1000;

const ALLOWED_DISCORD_IDS: [&str; 3] =
    ["187342661060001792", "188044055698079756", "1126322387240095824"];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Settings {
    pub limit: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self { limit: 5 }
    }
}

impl Settings {
    fn validate(&self) -> Result<(), ApiError> {
        if !(1..=7).contains(&self.limit) {
            return Err(ApiError::bad_request("Choose between 1 and 7 battles"));
        }

        Ok(())
    }

    fn from_claim(claim: &Document) -> Result<Self, ApiError> {
        let Ok(settings) = claim.get_document("overlaySettings") else {
            return Ok(Self::default());
        };

        mongodb::bson::from_document(settings.clone())
            .map_err(|error| ApiError::internal(error.to_string()))
    }
}

async fn owned_claim(
    state: &AppState,
    session: &AuthenticatedSession,
    governor: i64,
) -> Result<Document, ApiError> {
    if !ALLOWED_DISCORD_IDS.contains(&session.user.discord_id.as_str()) {
        return Err(ApiError::not_found("Overlay not found"));
    }

    state
        .reports_store
        .claimed_governors_collection()
        .find_one(doc! {"discordId": &session.user.discord_id, "governorId": governor})
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Claim not found"))
}

pub async fn settings(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let claim = owned_claim(&state, &session, governor).await?;

    Ok((
        NO_STORE,
        Json(serde_json::json!({
            "enabled": claim.get_str("overlayTokenHash").is_ok(),
            "settings": Settings::from_claim(&claim)?
        })),
    ))
}

pub async fn save(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: AuthenticatedSession,
    Json(settings): Json<Settings>,
) -> Result<impl IntoResponse, ApiError> {
    settings.validate()?;
    let claim = owned_claim(&state, &session, governor).await?;
    let settings_doc = mongodb::bson::to_document(&settings)
        .map_err(|error| ApiError::internal(error.to_string()))?;

    state
        .reports_store
        .claimed_governors_collection()
        .update_one(
            doc! {"_id": claim.get("_id").cloned()},
            doc! {"$set": {"overlaySettings": settings_doc}},
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((NO_STORE, Json(settings)))
}

fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);

    format!("ov_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn valid_token(token: &str) -> bool {
    token.strip_prefix("ov_").is_some_and(|value| {
        value.len() == 43
            && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    })
}

pub async fn rotate(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let claim = owned_claim(&state, &session, governor).await?;
    let token = generate_token();

    state
        .reports_store
        .claimed_governors_collection()
        .update_one(
            doc! {"_id": claim.get("_id").cloned()},
            doc! {"$set": {"overlayTokenHash": token_hash(&token)}},
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((NO_STORE, Json(serde_json::json!({"path": format!("/overlay/{token}")}))))
}

pub async fn revoke(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, ApiError> {
    let claim = owned_claim(&state, &session, governor).await?;

    state
        .reports_store
        .claimed_governors_collection()
        .update_one(
            doc! {"_id": claim.get("_id").cloned()},
            doc! {"$unset": {"overlayTokenHash": ""}},
        )
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((NO_STORE, Json(serde_json::json!({"enabled": false}))))
}

async fn token_claim(state: &AppState, token: &str) -> Result<Document, ApiError> {
    if !valid_token(token) {
        return Err(ApiError::not_found("Overlay not found"));
    }

    state
        .reports_store
        .claimed_governors_collection()
        .find_one(doc! {
            "overlayTokenHash": token_hash(token),
            "discordId": {"$in": ALLOWED_DISCORD_IDS.as_slice()}
        })
        .max_time(QUERY_TIMEOUT)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Overlay not found"))
}

pub async fn token_settings(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let claim = token_claim(&state, &token).await?;

    Ok((NO_STORE, Json(Settings::from_claim(&claim)?)))
}

pub async fn battles(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let claim = token_claim(&state, &token).await?;

    let governor = nested_i64(&claim, &["governorId"])
        .ok_or_else(|| ApiError::internal("Invalid overlay governor"))?;
    let settings = Settings::from_claim(&claim)?;
    let now = DateTime::now().timestamp_millis();

    let reports = store::fetch_reports(&state, governor, settings.limit, now).await?;
    let mut items = aggregate::aggregate(reports);

    items.retain(|item| item.expires_at > now);
    items.truncate(settings.limit);

    Ok((
        NO_STORE,
        Json(serde_json::json!({"items": items, "serverTime": now, "pollIntervalMs": 3000})),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unguessable_url_safe_tokens_and_distinct_hashes() {
        let a = generate_token();
        let b = generate_token();
        assert!(valid_token(&a));
        assert_ne!(a, b);
        assert_ne!(token_hash(&a), token_hash(&b));
        assert!(!valid_token("ov_test"));
        assert!(!valid_token("ov_../../etc/passwd"));
    }

    #[test]
    fn validates_limits() {
        for limit in 1..=7 {
            Settings { limit }.validate().expect("valid limit");
        }
        for limit in [0, 8] {
            assert!(Settings { limit }.validate().is_err());
        }
    }

    #[test]
    fn defaults_to_five_and_ignores_removed_settings() {
        assert_eq!(Settings::from_claim(&doc! {}).expect("default settings").limit, 5);
        let settings = Settings::from_claim(&doc! {"overlaySettings": {
            "limit": 3, "startAt": 1000, "primaryCommanderId": 575, "secondaryCommanderId": 579
        }})
        .expect("previously saved settings");
        assert_eq!(serde_json::to_value(settings).unwrap(), serde_json::json!({"limit": 3}));
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap().limit, 5);
    }
}
