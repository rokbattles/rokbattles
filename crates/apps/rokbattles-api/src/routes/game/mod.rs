use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;

use crate::{db::GameTranslation, error::ApiError, state::AppState};

const DEFAULT_VERSION: &str = "1.1.9.19";
const DEFAULT_LANGUAGE: &str = "en";

type TranslationResponse = BTreeMap<String, Option<String>>;

/// Build game routes.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/translate", get(get_translations).post(post_translations))
}

#[derive(Debug, Default, Deserialize)]
struct TranslationInput {
    key: Option<String>,
    ver: Option<String>,
    lang: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct TranslationRequest {
    keys: Vec<String>,
    version: String,
    language: String,
}

async fn get_translations(
    State(state): State<Arc<AppState>>,
    Query(input): Query<TranslationInput>,
) -> Result<Json<TranslationResponse>, ApiError> {
    translate(&state, input).await.map(Json)
}

async fn post_translations(
    State(state): State<Arc<AppState>>,
    Json(input): Json<TranslationInput>,
) -> Result<Json<TranslationResponse>, ApiError> {
    translate(&state, input).await.map(Json)
}

async fn translate(
    state: &AppState,
    input: TranslationInput,
) -> Result<TranslationResponse, ApiError> {
    let request = parse_request(input)?;
    let languages = state
        .game_localizations
        .languages_for_version(&request.version)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    validate_version_and_language(languages.as_ref(), &request.version, &request.language)?;

    let translations = state
        .game_localizations
        .find_translations(&request.version, &request.keys, &request.language)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(build_response(&request.keys, translations))
}

fn parse_request(input: TranslationInput) -> Result<TranslationRequest, ApiError> {
    let key = input
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| ApiError::bad_request("key is required"))?;
    let keys = key
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_uppercase)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(ApiError::bad_request("key is required"));
    }

    let version = input.ver.unwrap_or_else(|| DEFAULT_VERSION.to_string());
    let language = input.lang.unwrap_or_else(|| DEFAULT_LANGUAGE.to_string());
    Ok(TranslationRequest { keys, version, language })
}

fn validate_version_and_language(
    languages: Option<&BTreeSet<String>>,
    version: &str,
    language: &str,
) -> Result<(), ApiError> {
    let Some(languages) = languages else {
        return Err(ApiError::bad_request(format!("unsupported version: {version}")));
    };
    if !languages.contains(language) {
        return Err(ApiError::bad_request(format!("unsupported language: {language}")));
    }
    Ok(())
}

fn build_response(keys: &[String], translations: Vec<GameTranslation>) -> TranslationResponse {
    let mut response = keys.iter().cloned().map(|key| (key, None)).collect::<TranslationResponse>();
    for translation in translations {
        response.insert(translation.key, translation.value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_defaults_and_normalizes_multiple_keys() {
        let request = parse_request(TranslationInput {
            key: Some("lc_hero_name_2, LC_HERO_NAME_1,lc_hero_name_2".to_string()),
            ..TranslationInput::default()
        })
        .expect("request should be valid");

        assert_eq!(
            request,
            TranslationRequest {
                keys: vec!["LC_HERO_NAME_1".to_string(), "LC_HERO_NAME_2".to_string()],
                version: DEFAULT_VERSION.to_string(),
                language: DEFAULT_LANGUAGE.to_string(),
            }
        );
    }

    #[test]
    fn request_accepts_version_and_language_overrides() {
        let request = parse_request(TranslationInput {
            key: Some("LC_HERO_NAME_1".to_string()),
            ver: Some("1.2.0".to_string()),
            lang: Some("zh_CN".to_string()),
        })
        .expect("request should be valid");

        assert_eq!((request.version, request.language), ("1.2.0".to_string(), "zh_CN".to_string()));
    }

    #[test]
    fn request_rejects_missing_key() {
        let error = parse_request(TranslationInput::default()).expect_err("key should be required");

        assert!(matches!(error, ApiError::BadRequest(message) if message == "key is required"));
    }

    #[test]
    fn validation_rejects_unsupported_version() {
        let error = validate_version_and_language(None, "9.9.9", "en")
            .expect_err("version should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unsupported version: 9.9.9")
        );
    }

    #[test]
    fn validation_rejects_unsupported_language() {
        let languages = BTreeSet::from(["en".to_string()]);
        let error = validate_version_and_language(Some(&languages), DEFAULT_VERSION, "unknown")
            .expect_err("language should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unsupported language: unknown")
        );
    }

    #[test]
    fn response_keeps_invalid_keys_as_null() {
        let keys = vec!["LC_HERO_NAME_1".to_string(), "NOT-A-KEY".to_string()];
        let response = build_response(
            &keys,
            vec![GameTranslation {
                key: "LC_HERO_NAME_1".to_string(),
                value: Some("Julius Caesar".to_string()),
            }],
        );

        assert_eq!(
            response,
            BTreeMap::from([
                ("LC_HERO_NAME_1".to_string(), Some("Julius Caesar".to_string())),
                ("NOT-A-KEY".to_string(), None),
            ])
        );
    }
}
