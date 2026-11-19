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

use crate::{
    auth::{AuthenticatedSession, RequireRole, Role, roles},
    error::ApiError,
    routes::governor::bind::{find_accessible_claim, share_documents, share_for},
    state::AppState,
};

mod aggregate;
mod store;

const NO_STORE: [(&str, &str); 2] =
    [("Cache-Control", "no-store"), ("Referrer-Policy", "no-referrer")];
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const BATTLE_LIFETIME_MS: i64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Branding {
    IconWatermark,
    #[default]
    TextWatermark,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub(super) struct Settings {
    pub limit: usize,
    pub branding: Branding,
}

impl Default for Settings {
    fn default() -> Self {
        Self { limit: 5, branding: Branding::default() }
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

// A recipient's overlay lives on their share so revoking the share also removes
// its overlay token. The primary holder's overlay lives on the bind itself.
struct OverlayMembership {
    filter: Document,
    fields: Document,
    prefix: &'static str,
}

impl OverlayMembership {
    async fn load(
        state: &AppState,
        session: &AuthenticatedSession,
        governor: i64,
    ) -> Result<Self, ApiError> {
        let user = &session.user.discord_id;
        let claim = find_accessible_claim(&state.reports_store, user, governor, None).await?;
        let id = claim.get("_id").ok_or_else(|| ApiError::not_found("Claim not found"))?;

        if let Some(share) = share_for(&claim, user) {
            Ok(Self {
                filter: doc! {
                    "_id": id, "shares": { "$elemMatch": { "_id": share.get("_id"), "discordId": user } }
                },
                fields: share.clone(),
                prefix: "shares.$.",
            })
        } else {
            Ok(Self { filter: doc! { "_id": id, "discordId": user }, fields: claim, prefix: "" })
        }
    }

    async fn update(
        self,
        state: &AppState,
        operation: &str,
        fields: Document,
    ) -> Result<(), ApiError> {
        let fields = fields
            .into_iter()
            .map(|(key, value)| (format!("{}{key}", self.prefix), value))
            .collect::<Document>();

        let result = state
            .reports_store
            .claimed_governors_collection()
            .update_one(self.filter, doc! { operation: fields })
            .await
            .map_err(|error| ApiError::internal(error.to_string()))?;

        if result.matched_count != 1 {
            return Err(ApiError::not_found("Claim not found"));
        }

        Ok(())
    }
}

pub async fn settings(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: RequireRole<roles::ContentCreator>,
) -> Result<impl IntoResponse, ApiError> {
    let membership = OverlayMembership::load(&state, &session, governor).await?;

    Ok((
        NO_STORE,
        Json(serde_json::json!({
            "enabled": membership.fields.get_str("overlayTokenHash").is_ok(),
            "path": stored_path(&membership.fields),
            "settings": Settings::from_claim(&membership.fields)?
        })),
    ))
}

pub async fn save(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: RequireRole<roles::ContentCreator>,
    Json(settings): Json<Settings>,
) -> Result<impl IntoResponse, ApiError> {
    settings.validate()?;
    let membership = OverlayMembership::load(&state, &session, governor).await?;
    let settings_doc = mongodb::bson::to_document(&settings)
        .map_err(|error| ApiError::internal(error.to_string()))?;

    membership.update(&state, "$set", doc! { "overlaySettings": settings_doc }).await?;

    Ok((NO_STORE, Json(settings)))
}

fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

// Older overlays stored only the token hash. Rotating an overlay stores the
// token so the authenticated settings route can display the URL again.
fn stored_path(claim: &Document) -> Option<String> {
    let token = claim.get_str("overlayToken").ok()?;
    let hash = claim.get_str("overlayTokenHash").ok()?;

    (valid_token(token) && token_hash(token) == hash).then(|| format!("/overlay/{token}"))
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
    session: RequireRole<roles::ContentCreator>,
) -> Result<impl IntoResponse, ApiError> {
    let membership = OverlayMembership::load(&state, &session, governor).await?;
    let token = generate_token();

    membership
        .update(
            &state,
            "$set",
            doc! { "overlayTokenHash": token_hash(&token), "overlayToken": &token },
        )
        .await?;

    Ok((NO_STORE, Json(serde_json::json!({"path": format!("/overlay/{token}")}))))
}

pub async fn revoke(
    State(state): State<Arc<AppState>>,
    Path(governor): Path<i64>,
    session: RequireRole<roles::ContentCreator>,
) -> Result<impl IntoResponse, ApiError> {
    let membership = OverlayMembership::load(&state, &session, governor).await?;

    membership
        .update(&state, "$unset", doc! { "overlayTokenHash": "", "overlayToken": "" })
        .await?;

    Ok((NO_STORE, Json(serde_json::json!({"enabled": false}))))
}

async fn token_claim(state: &AppState, token: &str) -> Result<(i64, Settings), ApiError> {
    if !valid_token(token) {
        return Err(ApiError::not_found("Overlay not found"));
    }

    let hash = token_hash(token);
    let claim = state
        .reports_store
        .claimed_governors_collection()
        .find_one(doc! { "$or": [
            { "overlayTokenHash": &hash },
            { "shares.overlayTokenHash": &hash },
        ] })
        .max_time(QUERY_TIMEOUT)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Overlay not found"))?;

    let membership = if claim.get_str("overlayTokenHash").ok() == Some(&hash) {
        Some(&claim)
    } else {
        share_documents(&claim).find(|share| share.get_str("overlayTokenHash").ok() == Some(&hash))
    }
    .ok_or_else(|| ApiError::not_found("Overlay not found"))?;
    let owner_id = membership
        .get_str("discordId")
        .map_err(|_error| ApiError::not_found("Overlay not found"))?;

    // OBS requests present an overlay URL without a session. Check that the
    // account that issued the URL still has the content creator role.
    let owner = state
        .auth_store
        .find_user_by_discord_id(owner_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("Overlay not found"))?;

    if !owner.roles.contains(Role::ContentCreator) {
        return Err(ApiError::not_found("Overlay not found"));
    }

    let governor = nested_i64(&claim, &["governorId"])
        .ok_or_else(|| ApiError::internal("Invalid overlay governor"))?;
    Ok((governor, Settings::from_claim(membership)?))
}

pub async fn token_settings(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (_, settings) = token_claim(&state, &token).await?;

    Ok((NO_STORE, Json(settings)))
}

pub async fn battles(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (governor, settings) = token_claim(&state, &token).await?;
    let now = DateTime::now().timestamp_millis();

    let reports = store::fetch_reports(&state, governor, settings.limit, now).await?;
    let mut items = aggregate::aggregate(reports);

    items.retain(|item| item.expires_at > now);
    items.truncate(settings.limit);

    Ok((
        NO_STORE,
        Json(serde_json::json!({
            "items": items,
            "serverTime": now,
            "pollIntervalMs": 3000,
            "branding": settings.branding
        })),
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
    fn retrieves_only_a_valid_active_token_and_preserves_hash_only_claims() {
        let token = generate_token();
        let hash = token_hash(&token);
        let legacy = doc! {"overlayTokenHash": &hash};
        assert_eq!(stored_path(&legacy), None);
        assert_eq!(legacy.get_str("overlayTokenHash").unwrap(), hash);

        let active = doc! {"overlayTokenHash": &hash, "overlayToken": &token};
        assert_eq!(stored_path(&active), Some(format!("/overlay/{token}")));

        for claim in [
            doc! {},
            doc! {"overlayToken": &token},
            doc! {"overlayTokenHash": token_hash(&generate_token()), "overlayToken": &token},
            doc! {"overlayTokenHash": token_hash("invalid"), "overlayToken": "invalid"},
        ] {
            assert_eq!(stored_path(&claim), None);
        }
    }

    #[test]
    fn validates_limits() {
        for limit in 1..=7 {
            Settings { limit, ..Settings::default() }.validate().expect("valid limit");
        }
        for limit in [0, 8] {
            assert!(Settings { limit, ..Settings::default() }.validate().is_err());
        }
    }

    #[test]
    fn defaults_to_five_and_ignores_removed_settings() {
        assert_eq!(Settings::from_claim(&doc! {}).expect("default settings").limit, 5);
        let settings = Settings::from_claim(&doc! {"overlaySettings": {
            "limit": 3, "startAt": 1000, "primaryCommanderId": 575, "secondaryCommanderId": 579
        }})
        .expect("previously saved settings");
        assert_eq!(
            serde_json::to_value(settings).unwrap(),
            serde_json::json!({"limit": 3, "branding": "text_watermark"})
        );
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap().limit, 5);
    }

    #[test]
    fn defaults_missing_branding_to_text_watermark() {
        for claim in [doc! {}, doc! {"overlaySettings": {"limit": 7}}] {
            let settings = Settings::from_claim(&claim).expect("existing settings");

            assert_eq!(settings.branding, Branding::TextWatermark);
        }

        let settings: Settings = serde_json::from_str("{}").expect("default settings");
        assert_eq!(settings.branding, Branding::TextWatermark);
    }

    #[test]
    fn preserves_branding_through_json_and_stored_settings() {
        for branding in ["icon_watermark", "text_watermark"] {
            let input = serde_json::json!({"limit": 3, "branding": branding});
            let settings: Settings = serde_json::from_value(input.clone()).expect("valid settings");
            let stored = mongodb::bson::to_document(&settings).expect("stored settings");
            let restored =
                Settings::from_claim(&doc! {"overlaySettings": stored}).expect("restored settings");

            assert_eq!(serde_json::to_value(restored).unwrap(), input);
        }
    }

    #[test]
    fn rejects_unsupported_branding() {
        for branding in ["header", "footer", "watermark", "unknown"] {
            serde_json::from_value::<Settings>(
                serde_json::json!({"limit": 5, "branding": branding}),
            )
            .expect_err("unsupported branding should be rejected");
        }
    }
}
