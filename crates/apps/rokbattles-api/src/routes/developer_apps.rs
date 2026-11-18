//! Developer Program app registration and settings.

use std::{sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware,
    response::IntoResponse,
    routing::{get, post, put},
};
use mongodb::bson::oid::ObjectId;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{RequireRole, roles::DeveloperProgram},
    db::{
        DeveloperApp, DeveloperAppGeneralSettings, DeveloperAppSettings, DeveloperAppUpdate,
        DeveloperAppsStore,
    },
    error::ApiError,
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

const MAX_NAME_LENGTH: usize = 100;
const MAX_DESCRIPTION_LENGTH: usize = 1000;
const MAX_URL_LENGTH: usize = 2048;
const MAX_REDIRECT_URIS: usize = 10;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateApp {
    name: String,
    #[serde(default)]
    accept_terms: bool,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "section",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum UpdateSection {
    General(DeveloperAppGeneralSettings),
    Authorization { redirect_uris: Vec<String> },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppResponse {
    id: String,
    terms_accepted_at: i64,
    user_agent: String,
    #[serde(flatten)]
    settings: DeveloperAppSettings,
}

impl From<DeveloperApp> for AppResponse {
    fn from(app: DeveloperApp) -> Self {
        let id = app.id.to_hex();
        Self {
            user_agent: user_agent(&id, &app.settings.name),
            id,
            terms_accepted_at: app.terms_accepted_at.timestamp_millis(),
            settings: app.settings,
        }
    }
}

pub(super) fn router(mode: RateLimitMode) -> Router<Arc<AppState>> {
    let limiter = Arc::new(RateLimiter::new(mode, 10, Duration::from_secs(6)));
    let write_routes = Router::new()
        .route("/developer/apps", post(create))
        .route("/developer/apps/{id}", put(update).patch(update_section))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(middleware::from_fn_with_state(limiter, rate_limit::enforce));

    Router::new()
        .route("/developer/apps", get(list))
        .route("/developer/apps/{id}", get(details))
        .merge(write_routes)
        .layer(middleware::from_fn(async |request: Request, next: middleware::Next| {
            let mut response = next.run(request).await;
            response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}

fn store(state: &AppState) -> Result<&DeveloperAppsStore, ApiError> {
    state.developer_apps.as_ref().ok_or_else(|| ApiError::internal("developer apps unavailable"))
}

async fn list(
    session: RequireRole<DeveloperProgram>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let apps = store(&state)?.list(&session.user.discord_id).await.map_err(storage_error)?;
    Ok(Json(apps.into_iter().map(AppResponse::from).collect::<Vec<_>>()))
}

async fn details(
    session: RequireRole<DeveloperProgram>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let app = store(&state)?
        .find(&session.user.discord_id, parse_app_id(&id)?)
        .await
        .map_err(storage_error)?
        .ok_or_else(app_not_found)?;
    Ok(Json(AppResponse::from(app)))
}

async fn create(
    session: RequireRole<DeveloperProgram>,
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateApp>,
) -> Result<impl IntoResponse, ApiError> {
    let settings = input.validate()?;
    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;

    let app =
        store(&state)?.insert(&session.user.discord_id, settings).await.map_err(storage_error)?;
    Ok((StatusCode::CREATED, Json(AppResponse::from(app))))
}

async fn update(
    session: RequireRole<DeveloperProgram>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(settings): Json<DeveloperAppSettings>,
) -> Result<impl IntoResponse, ApiError> {
    let update = DeveloperAppUpdate::All(validate_settings(settings)?);
    save_update(&state, &session, parse_app_id(&id)?, update).await
}

async fn update_section(
    session: RequireRole<DeveloperProgram>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(section): Json<UpdateSection>,
) -> Result<Json<AppResponse>, ApiError> {
    let update = match section {
        UpdateSection::General(general) => DeveloperAppUpdate::General(validate_general(general)?),
        UpdateSection::Authorization { redirect_uris } => {
            validate_redirect_uris(&redirect_uris)?;
            DeveloperAppUpdate::RedirectUris(redirect_uris)
        }
    };
    save_update(&state, &session, parse_app_id(&id)?, update).await
}

async fn save_update(
    state: &AppState,
    session: &RequireRole<DeveloperProgram>,
    id: ObjectId,
    update: DeveloperAppUpdate,
) -> Result<Json<AppResponse>, ApiError> {
    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;

    let app = store(state)?
        .update(&session.user.discord_id, id, update)
        .await
        .map_err(storage_error)?
        .ok_or_else(app_not_found)?;
    Ok(Json(AppResponse::from(app)))
}

fn storage_error(error: mongodb::error::Error) -> ApiError {
    tracing::error!(%error, "developer app database operation failed");
    ApiError::internal("developer app database operation failed")
}

fn app_not_found() -> ApiError {
    ApiError::not_found("App not found.")
}

fn parse_app_id(id: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(id).map_err(|_error| app_not_found())
}

impl CreateApp {
    fn validate(self) -> Result<DeveloperAppSettings, ApiError> {
        if !self.accept_terms {
            return Err(ApiError::bad_request(
                "Accept the ROK Battles Terms of Service to create an app.",
            ));
        }

        Ok(DeveloperAppGeneralSettings {
            name: validate_name(self.name)?,
            description: None,
            terms_of_service_url: None,
            privacy_policy_url: None,
        }
        .with_redirect_uris(Vec::new()))
    }
}

fn validate_name(name: String) -> Result<String, ApiError> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_NAME_LENGTH
        || name.chars().any(char::is_control)
    {
        return Err(ApiError::bad_request(
            "Enter an app name of 1 to 100 characters without control characters.",
        ));
    }
    Ok(name.to_owned())
}

fn validate_settings(settings: DeveloperAppSettings) -> Result<DeveloperAppSettings, ApiError> {
    let DeveloperAppSettings {
        name,
        description,
        terms_of_service_url,
        privacy_policy_url,
        redirect_uris,
    } = settings;
    let general = validate_general(DeveloperAppGeneralSettings {
        name,
        description,
        terms_of_service_url,
        privacy_policy_url,
    })?;
    validate_redirect_uris(&redirect_uris)?;
    Ok(general.with_redirect_uris(redirect_uris))
}

fn validate_general(
    mut settings: DeveloperAppGeneralSettings,
) -> Result<DeveloperAppGeneralSettings, ApiError> {
    settings.name = validate_name(settings.name)?;
    settings.description = settings.description.and_then(trim_optional);
    if settings
        .description
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_DESCRIPTION_LENGTH)
    {
        return Err(ApiError::bad_request("Enter a description of 1,000 characters or fewer."));
    }
    settings.terms_of_service_url =
        validate_legal_url(settings.terms_of_service_url, "Terms of Service URL")?;
    settings.privacy_policy_url =
        validate_legal_url(settings.privacy_policy_url, "Privacy Policy URL")?;

    Ok(settings)
}

fn validate_redirect_uris(redirect_uris: &[String]) -> Result<(), ApiError> {
    if redirect_uris.len() > MAX_REDIRECT_URIS {
        return Err(ApiError::bad_request("Register up to 10 redirect URIs."));
    }
    let mut unique = std::collections::BTreeSet::new();
    for (index, uri) in redirect_uris.iter().enumerate() {
        if !is_redirect_uri(uri) {
            return Err(ApiError::bad_request(format!(
                "Redirect URI {} must use HTTPS or a loopback HTTP address and contain no credentials, fragments, or wildcards.",
                index + 1
            )));
        }
        if !unique.insert(uri) {
            return Err(ApiError::bad_request("Each redirect URI must be unique."));
        }
    }
    Ok(())
}

fn trim_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn validate_legal_url(value: Option<String>, label: &str) -> Result<Option<String>, ApiError> {
    let value = value.and_then(trim_optional);
    if let Some(uri) = value.as_deref()
        && !matches!(parse_web_uri(uri), Some((url, _)) if url.scheme() == "https")
    {
        return Err(ApiError::bad_request(format!("Enter an HTTPS {label} without credentials.")));
    }
    Ok(value)
}

fn is_redirect_uri(uri: &str) -> bool {
    let Some((url, authority)) = parse_web_uri(uri) else {
        return false;
    };
    if url.fragment().is_some() || uri.contains('*') {
        return false;
    }
    url.scheme() == "https"
        || (url.scheme() == "http"
            && ["127.0.0.1", "[::1]"].iter().any(|host| {
                authority == *host
                    || authority.strip_prefix(host).is_some_and(|port| port.starts_with(':'))
            }))
}

fn parse_web_uri(uri: &str) -> Option<(Url, &str)> {
    let bytes = uri.as_bytes();
    if bytes.len() > MAX_URL_LENGTH
        || !bytes.iter().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric()
                || b"-._~:/?#[]@!$&'()*+,;=".contains(byte)
                || (*byte == b'%'
                    && bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
                    && bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit))
        })
    {
        return None;
    }

    let (_, rest) = uri.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains(['@', '%']) || authority.ends_with(':') {
        return None;
    }
    let url = Url::parse(uri).ok()?;
    if !matches!(url.scheme(), "http" | "https") || !url.has_host() {
        return None;
    }
    Some((url, authority))
}

fn user_agent(id: &str, name: &str) -> String {
    // Use a single HTTP product token and reserve comment delimiters for the optional suffix.
    let name: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("ROKBattles/{id} ({name})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_explicit_terms_acceptance() {
        let request: CreateApp = serde_json::from_str(r#"{"name":"Roster Tool"}"#).unwrap();
        assert!(matches!(request.validate(), Err(ApiError::BadRequest(_))));
    }

    #[test]
    fn registers_with_only_name_and_terms() {
        let request = CreateApp { name: " Roster Tool ".into(), accept_terms: true };
        let settings = request.validate().unwrap();
        assert_eq!(settings.name, "Roster Tool");
        assert!(settings.description.is_none());
        assert!(settings.terms_of_service_url.is_none());
        assert!(settings.privacy_policy_url.is_none());
        assert!(settings.redirect_uris.is_empty());
    }

    #[test]
    fn rejects_empty_names_and_owner_fields() {
        validate_name("   ".into()).unwrap_err();
        validate_name("Line\nBreak".into()).unwrap_err();
        serde_json::from_str::<CreateApp>(r#"{"name":"App","acceptTerms":true,"userId":"other"}"#)
            .unwrap_err();
    }

    #[test]
    fn preserves_valid_redirect_uri_spelling() {
        let uri = "https://Example.COM:443/CallBack?tenant=a%20b";
        let mut settings = CreateApp { name: "App".into(), accept_terms: true }.validate().unwrap();
        settings.redirect_uris = vec![
            uri.into(),
            "http://127.0.0.1:54321/callback".into(),
            "http://[::1]/callback".into(),
        ];
        assert_eq!(validate_settings(settings).unwrap().redirect_uris[0], uri);
    }

    #[test]
    fn rejects_invalid_redirect_uris() {
        for uri in [
            "/callback",
            "https:///example.com/callback",
            "https://example.com/call back",
            "https://user:pass@example.com/",
            "https://@example.com/",
            "https://example.com/#",
            "https://example.com/*",
            "http://example.com/",
            "http://localhost/",
            "http://127.1/",
            "javascript:alert(1)",
            "https://example.com/%zz",
            "https://example.com\\callback",
        ] {
            assert!(!is_redirect_uri(uri), "{uri}");
        }
    }

    #[test]
    fn rejects_duplicate_redirect_uris_and_invalid_legal_urls() {
        let mut settings = CreateApp { name: "App".into(), accept_terms: true }.validate().unwrap();
        settings.redirect_uris = vec!["https://example.com/callback".into(); 2];
        validate_settings(settings).unwrap_err();
        validate_legal_url(Some("http://127.0.0.1/terms".into()), "Terms URL").unwrap_err();
    }

    #[test]
    fn section_updates_accept_only_fields_in_the_requested_section() {
        let general =
            serde_json::from_str::<UpdateSection>(r#"{"section":"general","name":"Test"}"#)
                .unwrap();
        assert!(matches!(general, UpdateSection::General(settings) if settings.name == "Test"));
        let redirects = serde_json::from_str::<UpdateSection>(
            r#"{"section":"authorization","redirectUris":[]}"#,
        )
        .unwrap();
        assert!(
            matches!(redirects, UpdateSection::Authorization { redirect_uris } if redirect_uris.is_empty())
        );
        serde_json::from_str::<UpdateSection>(
            r#"{"section":"general","name":"Test","redirectUris":[]}"#,
        )
        .unwrap_err();
    }

    #[test]
    fn formats_user_agent_with_safe_comment_characters() {
        assert_eq!(user_agent("app-id", "Tool (β)/2;"), "ROKBattles/app-id (Tool ____2_)");
    }
}
