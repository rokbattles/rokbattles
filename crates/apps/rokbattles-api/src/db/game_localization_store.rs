use std::collections::BTreeSet;

use futures::TryStreamExt;
use mongodb::{
    Collection, IndexModel,
    bson::{Bson, Document, doc},
    options::IndexOptions,
};

/// One translated value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTranslation {
    pub key: String,
    pub value: Option<String>,
}

/// Translation lookup errors.
#[derive(Debug, thiserror::Error)]
pub enum GameLocalizationStoreError {
    #[error("database error: {0}")]
    Database(#[from] mongodb::error::Error),
    #[error("invalid localization document: {0}")]
    InvalidDocument(String),
}

/// Versioned translation lookup service.
#[derive(Debug, Clone)]
pub struct GameLocalizationStore {
    localizations: Collection<Document>,
}

impl GameLocalizationStore {
    /// Create a translation lookup service.
    pub fn new(db: mongodb::Database) -> Self {
        Self { localizations: db.collection("g_rok_game_lc") }
    }

    /// Prepare the service for requests.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.localizations
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "version": 1, "key": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        Ok(())
    }

    pub async fn languages_for_version(
        &self,
        version: &str,
    ) -> Result<Option<BTreeSet<String>>, GameLocalizationStoreError> {
        let document = self
            .localizations
            .find_one(doc! { "version": version })
            .projection(doc! { "_id": 0, "values": 1 })
            .await?;
        let Some(document) = document else {
            return Ok(None);
        };
        let values = document
            .get_document("values")
            .map_err(|error| GameLocalizationStoreError::InvalidDocument(error.to_string()))?;
        Ok(Some(values.keys().cloned().collect()))
    }

    pub async fn find_translations(
        &self,
        version: &str,
        keys: &[String],
        language: &str,
    ) -> Result<Vec<GameTranslation>, GameLocalizationStoreError> {
        let value_path = format!("values.{language}");
        let mut projection = doc! { "_id": 0, "key": 1 };
        projection.insert(&value_path, 1);
        let key_values = keys.iter().cloned().map(Bson::String).collect::<Vec<_>>();
        let cursor = self
            .localizations
            .find(doc! {
                "version": version,
                "key": { "$in": key_values },
            })
            .projection(projection)
            .await?;
        let documents: Vec<Document> = cursor.try_collect().await?;

        documents.into_iter().map(|document| map_translation(document, language)).collect()
    }
}

fn map_translation(
    document: Document,
    language: &str,
) -> Result<GameTranslation, GameLocalizationStoreError> {
    let key = document
        .get_str("key")
        .map(str::to_owned)
        .map_err(|error| GameLocalizationStoreError::InvalidDocument(error.to_string()))?;
    let values = document
        .get_document("values")
        .map_err(|error| GameLocalizationStoreError::InvalidDocument(error.to_string()))?;
    let value = match values.get(language) {
        Some(Bson::String(value)) => Some(value.clone()),
        Some(Bson::Null) | None => None,
        Some(value) => {
            return Err(GameLocalizationStoreError::InvalidDocument(format!(
                "{key}.{language} must be a string or null, got {value:?}"
            )));
        }
    };

    Ok(GameTranslation { key, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_translation_reads_requested_language() {
        let document = doc! {
            "key": "LC_HERO_NAME_1",
            "values": { "en": "Julius Caesar", "ja": "ユリウス・カエサル" },
        };

        let translation = map_translation(document, "ja").expect("document should be valid");

        assert_eq!(
            translation,
            GameTranslation {
                key: "LC_HERO_NAME_1".to_string(),
                value: Some("ユリウス・カエサル".to_string()),
            }
        );
    }

    #[test]
    fn map_translation_preserves_null_value() {
        let document = doc! {
            "key": "LC_HERO_NAME_MISSING",
            "values": { "en": Bson::Null },
        };

        let translation = map_translation(document, "en").expect("document should be valid");

        assert_eq!(translation.value, None);
    }
}
