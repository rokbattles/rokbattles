use std::collections::BTreeSet;

use axum::{
    Json,
    extract::{Query, State},
};
use mongodb::bson::{Bson, Document, doc, to_bson};
use serde::Deserialize;

use super::DEFAULT_VERSION;
use crate::{
    db::{GameExcelDataRepository, GameExcelDataSheet},
    error::ApiError,
    state::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub(super) struct ExcelDataGetInput {
    sheet: Option<String>,
    ver: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct ExcelDataPostInput {
    #[serde(alias = "sheet")]
    key: Option<String>,
    #[serde(alias = "version")]
    ver: Option<String>,
    #[serde(default)]
    fields: Vec<ExcelDataFieldInput>,
}

#[derive(Debug, Deserialize)]
struct ExcelDataFieldInput {
    field: String,
    op: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExcelDataOperator {
    Eq,
    Ne,
}

#[derive(Debug, PartialEq)]
struct ExcelDataField {
    field: String,
    op: ExcelDataOperator,
    value: Bson,
}

#[derive(Debug, PartialEq)]
struct ExcelDataRequest {
    sheet: String,
    version: String,
    fields: Vec<ExcelDataField>,
}

pub(super) async fn get(
    State(state): State<std::sync::Arc<AppState>>,
    Query(input): Query<ExcelDataGetInput>,
) -> Result<Json<Vec<Document>>, ApiError> {
    let request = parse_get_request(input)?;
    query(&state.game_excel_data, request).await.map(Json)
}

pub(super) async fn post(
    State(state): State<std::sync::Arc<AppState>>,
    Json(input): Json<ExcelDataPostInput>,
) -> Result<Json<Vec<Document>>, ApiError> {
    let request = parse_post_request(input)?;
    query(&state.game_excel_data, request).await.map(Json)
}

async fn query(
    repository: &impl GameExcelDataRepository,
    request: ExcelDataRequest,
) -> Result<Vec<Document>, ApiError> {
    let metadata =
        repository.find_sheet(&request.version, &request.sheet).await.map_err(internal_error)?;
    let Some(metadata) = metadata else {
        if repository.version_exists(&request.version).await.map_err(internal_error)? {
            return Err(ApiError::not_found(format!("unknown sheet: {}", request.sheet)));
        }
        return Err(ApiError::bad_request(format!("unsupported version: {}", request.version)));
    };

    validate_fields(&metadata, &request.fields)?;
    repository
        .find_rows(&request.version, &request.sheet, build_predicates(&request.fields))
        .await
        .map_err(internal_error)
}

fn parse_get_request(input: ExcelDataGetInput) -> Result<ExcelDataRequest, ApiError> {
    let sheet = required_value(input.sheet, "sheet")?;
    Ok(ExcelDataRequest { sheet, version: version_or_default(input.ver), fields: Vec::new() })
}

fn parse_post_request(input: ExcelDataPostInput) -> Result<ExcelDataRequest, ApiError> {
    let sheet = required_value(input.key, "key")?;
    let fields = input.fields.into_iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;
    Ok(ExcelDataRequest { sheet, version: version_or_default(input.ver), fields })
}

fn required_value(value: Option<String>, name: &str) -> Result<String, ApiError> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{name} is required")))
}

fn version_or_default(version: Option<String>) -> String {
    version.unwrap_or_else(|| DEFAULT_VERSION.to_string())
}

fn parse_field(input: ExcelDataFieldInput) -> Result<ExcelDataField, ApiError> {
    let field = required_value(Some(input.field), "field")?;
    let op = match input.op.trim().to_ascii_lowercase().as_str() {
        "eq" => ExcelDataOperator::Eq,
        "ne" => ExcelDataOperator::Ne,
        operator => {
            return Err(ApiError::bad_request(format!("unsupported operator: {operator}")));
        }
    };
    let value = to_bson(&input.value)
        .map_err(|error| ApiError::bad_request(format!("invalid filter value: {error}")))?;
    Ok(ExcelDataField { field, op, value })
}

fn validate_fields(
    metadata: &GameExcelDataSheet,
    fields: &[ExcelDataField],
) -> Result<(), ApiError> {
    let known_fields =
        metadata.columns.iter().map(|column| column.name.as_str()).collect::<BTreeSet<_>>();
    for field in fields {
        if !known_fields.contains(field.field.as_str()) {
            return Err(ApiError::bad_request(format!(
                "unknown field for {}: {}",
                metadata.sheet, field.field
            )));
        }
    }
    Ok(())
}

fn build_predicates(fields: &[ExcelDataField]) -> Document {
    if fields.is_empty() {
        return Document::new();
    }

    let predicates = fields
        .iter()
        .map(|field| {
            let path = format!("data.{}", field.field);
            let predicate = match field.op {
                ExcelDataOperator::Eq => doc! { path: field.value.clone() },
                ExcelDataOperator::Ne => doc! { path: { "$ne": field.value.clone() } },
            };
            Bson::Document(predicate)
        })
        .collect::<Vec<_>>();
    doc! { "$and": predicates }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use futures::{FutureExt, future::BoxFuture};

    use super::*;
    use crate::db::{GameExcelDataColumn, GameExcelDataStoreError};

    struct MockRepository {
        sheet: Option<GameExcelDataSheet>,
        version_exists: bool,
        rows: Vec<Document>,
        predicates: Mutex<Option<Document>>,
    }

    impl MockRepository {
        fn with_sheet() -> Self {
            Self {
                sheet: Some(GameExcelDataSheet {
                    version: DEFAULT_VERSION.to_string(),
                    sheet: "alliance_armory_const".to_string(),
                    columns: vec![
                        GameExcelDataColumn {
                            name: "ID".to_string(),
                            value_type: "integer".to_string(),
                        },
                        GameExcelDataColumn {
                            name: "Key".to_string(),
                            value_type: "string".to_string(),
                        },
                    ],
                }),
                version_exists: true,
                rows: vec![doc! { "ID": 1_i32, "Key": "DONATE_SCORE_1" }],
                predicates: Mutex::new(None),
            }
        }
    }

    impl GameExcelDataRepository for MockRepository {
        fn find_sheet<'a>(
            &'a self,
            _version: &'a str,
            _sheet: &'a str,
        ) -> BoxFuture<'a, Result<Option<GameExcelDataSheet>, GameExcelDataStoreError>> {
            async move { Ok(self.sheet.clone()) }.boxed()
        }

        fn version_exists<'a>(
            &'a self,
            _version: &'a str,
        ) -> BoxFuture<'a, Result<bool, GameExcelDataStoreError>> {
            async move { Ok(self.version_exists) }.boxed()
        }

        fn find_rows<'a>(
            &'a self,
            _version: &'a str,
            _sheet: &'a str,
            predicates: Document,
        ) -> BoxFuture<'a, Result<Vec<Document>, GameExcelDataStoreError>> {
            async move {
                *self.predicates.lock().expect("mutex should not be poisoned") = Some(predicates);
                Ok(self.rows.clone())
            }
            .boxed()
        }
    }

    #[test]
    fn get_request_requires_sheet_and_uses_default_version() {
        let error =
            parse_get_request(ExcelDataGetInput::default()).expect_err("sheet should be required");
        assert!(matches!(error, ApiError::BadRequest(message) if message == "sheet is required"));

        let request = parse_get_request(ExcelDataGetInput {
            sheet: Some("alliance_armory_const".to_string()),
            ver: None,
        })
        .expect("request should be valid");
        assert_eq!(request.version, DEFAULT_VERSION);
    }

    #[test]
    fn post_request_accepts_eq_and_ne_filters() {
        let request = parse_post_request(ExcelDataPostInput {
            key: Some("alliance_armory_const".to_string()),
            ver: Some("1.2.0".to_string()),
            fields: vec![
                ExcelDataFieldInput {
                    field: "ID".to_string(),
                    op: "eq".to_string(),
                    value: serde_json::json!(1),
                },
                ExcelDataFieldInput {
                    field: "Key".to_string(),
                    op: "NE".to_string(),
                    value: serde_json::json!("DONATE_SCORE_2"),
                },
            ],
        })
        .expect("request should be valid");

        assert_eq!(request.version, "1.2.0");
        assert_eq!(request.fields[0].op, ExcelDataOperator::Eq);
        assert_eq!(request.fields[1].op, ExcelDataOperator::Ne);
    }

    #[test]
    fn post_request_rejects_unknown_operator() {
        let error = parse_post_request(ExcelDataPostInput {
            key: Some("alliance_armory_const".to_string()),
            fields: vec![ExcelDataFieldInput {
                field: "ID".to_string(),
                op: "gt".to_string(),
                value: serde_json::json!(1),
            }],
            ..ExcelDataPostInput::default()
        })
        .expect_err("operator should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unsupported operator: gt")
        );
    }

    #[tokio::test]
    async fn query_returns_rows_and_builds_anded_predicates() {
        let repository = MockRepository::with_sheet();
        let request = ExcelDataRequest {
            sheet: "alliance_armory_const".to_string(),
            version: DEFAULT_VERSION.to_string(),
            fields: vec![ExcelDataField {
                field: "ID".to_string(),
                op: ExcelDataOperator::Ne,
                value: Bson::Int32(2),
            }],
        };

        let rows = query(&repository, request).await.expect("query should succeed");

        assert_eq!(rows, repository.rows);
        assert_eq!(
            *repository.predicates.lock().expect("mutex should not be poisoned"),
            Some(doc! { "$and": [{ "data.ID": { "$ne": 2_i32 } }] })
        );
    }

    #[tokio::test]
    async fn query_rejects_unknown_field() {
        let repository = MockRepository::with_sheet();
        let request = ExcelDataRequest {
            sheet: "alliance_armory_const".to_string(),
            version: DEFAULT_VERSION.to_string(),
            fields: vec![ExcelDataField {
                field: "Unknown".to_string(),
                op: ExcelDataOperator::Eq,
                value: Bson::Int32(1),
            }],
        };

        let error = query(&repository, request).await.expect_err("field should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unknown field for alliance_armory_const: Unknown")
        );
    }

    #[tokio::test]
    async fn query_returns_not_found_for_unknown_sheet() {
        let repository = MockRepository {
            sheet: None,
            version_exists: true,
            rows: Vec::new(),
            predicates: Mutex::new(None),
        };
        let request = ExcelDataRequest {
            sheet: "missing".to_string(),
            version: DEFAULT_VERSION.to_string(),
            fields: Vec::new(),
        };

        let error = query(&repository, request).await.expect_err("sheet should be missing");

        assert!(
            matches!(error, ApiError::NotFound(message) if message == "unknown sheet: missing")
        );
    }

    #[tokio::test]
    async fn query_rejects_unsupported_version() {
        let repository = MockRepository {
            sheet: None,
            version_exists: false,
            rows: Vec::new(),
            predicates: Mutex::new(None),
        };
        let request = ExcelDataRequest {
            sheet: "alliance_armory_const".to_string(),
            version: "9.9.9".to_string(),
            fields: Vec::new(),
        };

        let error = query(&repository, request).await.expect_err("version should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unsupported version: 9.9.9")
        );
    }
}
