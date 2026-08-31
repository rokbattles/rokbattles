use futures::{FutureExt, TryStreamExt, future::BoxFuture};
use mongodb::{
    Collection, IndexModel,
    bson::{Document, doc, from_document},
    options::IndexOptions,
};
use serde::Deserialize;

/// One queryable field.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GameQueryColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: String,
}

/// Fields available for a versioned query.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GameQuerySheet {
    pub version: String,
    pub sheet: String,
    pub columns: Vec<GameQueryColumn>,
}

/// Game-query errors.
#[derive(Debug, thiserror::Error)]
pub enum GameQueryStoreError {
    #[error("database error: {0}")]
    Database(#[from] mongodb::error::Error),
    #[error("invalid game-data document: {0}")]
    InvalidDocument(String),
}

/// Versioned game-query operations.
pub trait GameQueryRepository: Send + Sync {
    fn find_sheet<'a>(
        &'a self,
        version: &'a str,
        sheet: &'a str,
    ) -> BoxFuture<'a, Result<Option<GameQuerySheet>, GameQueryStoreError>>;

    fn version_exists<'a>(
        &'a self,
        version: &'a str,
    ) -> BoxFuture<'a, Result<bool, GameQueryStoreError>>;

    fn find_rows<'a>(
        &'a self,
        version: &'a str,
        sheet: &'a str,
        predicates: Document,
    ) -> BoxFuture<'a, Result<Vec<Document>, GameQueryStoreError>>;
}

/// Versioned game-query service.
#[derive(Debug, Clone)]
pub struct GameQueryStore {
    sheets: Collection<Document>,
    rows: Collection<Document>,
}

impl GameQueryStore {
    /// Create a game-query service.
    pub fn new(db: mongodb::Database) -> Self {
        Self {
            sheets: db.collection("g_rok_game_excel_sheets"),
            rows: db.collection("g_rok_game_excel_rows"),
        }
    }

    /// Prepare the service for requests.
    pub async fn ensure_indexes(&self) -> mongodb::error::Result<()> {
        self.sheets
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "version": 1, "sheet": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        self.rows
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "version": 1, "sheet": 1, "ordinal": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await?;
        Ok(())
    }
}

impl GameQueryRepository for GameQueryStore {
    fn find_sheet<'a>(
        &'a self,
        version: &'a str,
        sheet: &'a str,
    ) -> BoxFuture<'a, Result<Option<GameQuerySheet>, GameQueryStoreError>> {
        async move {
            let document = self
                .sheets
                .find_one(doc! { "version": version, "sheet": sheet })
                .projection(doc! { "_id": 0 })
                .await?;
            document.map(map_sheet).transpose()
        }
        .boxed()
    }

    fn version_exists<'a>(
        &'a self,
        version: &'a str,
    ) -> BoxFuture<'a, Result<bool, GameQueryStoreError>> {
        async move {
            Ok(self
                .sheets
                .find_one(doc! { "version": version })
                .projection(doc! { "_id": 1 })
                .await?
                .is_some())
        }
        .boxed()
    }

    fn find_rows<'a>(
        &'a self,
        version: &'a str,
        sheet: &'a str,
        predicates: Document,
    ) -> BoxFuture<'a, Result<Vec<Document>, GameQueryStoreError>> {
        async move {
            let mut filter = doc! { "version": version, "sheet": sheet };
            filter.extend(predicates);
            let cursor = self
                .rows
                .find(filter)
                .projection(doc! { "_id": 0, "data": 1 })
                .sort(doc! { "ordinal": 1 })
                .await?;
            let documents: Vec<Document> = cursor.try_collect().await?;
            documents.into_iter().map(map_row).collect()
        }
        .boxed()
    }
}

fn map_sheet(document: Document) -> Result<GameQuerySheet, GameQueryStoreError> {
    from_document(document).map_err(|error| GameQueryStoreError::InvalidDocument(error.to_string()))
}

fn map_row(mut document: Document) -> Result<Document, GameQueryStoreError> {
    document
        .remove("data")
        .ok_or_else(|| GameQueryStoreError::InvalidDocument("data is required".to_string()))?
        .as_document()
        .cloned()
        .ok_or_else(|| GameQueryStoreError::InvalidDocument("data must be a document".to_string()))
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::*;

    #[test]
    fn map_sheet_reads_schema_metadata() {
        let sheet = map_sheet(doc! {
            "version": "1.1.11.25",
            "sheet": "alliance_armory_const",
            "columns": [
                { "name": "ID", "type": "integer" },
                { "name": "Key", "type": "string" },
            ],
        })
        .expect("metadata should be valid");

        assert_eq!(sheet.columns[1].name, "Key");
    }

    #[test]
    fn map_row_returns_only_sheet_data() {
        let row = map_row(doc! {
            "data": { "ID": 1_i32, "Key": "DONATE_SCORE_1" },
        })
        .expect("row should be valid");

        assert_eq!(row.get("ID"), Some(&Bson::Int32(1)));
        assert_eq!(row.get_str("Key"), Ok("DONATE_SCORE_1"));
    }
}
