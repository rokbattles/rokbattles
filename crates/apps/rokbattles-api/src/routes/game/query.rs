use std::collections::BTreeSet;

use axum::{
    Json,
    extract::{Query, State},
};
use mongodb::bson::{Bson, Document, doc, to_bson};
use serde::Deserialize;

use super::DEFAULT_VERSION;
use crate::{
    db::{GameQueryRepository, GameQuerySheet},
    error::ApiError,
    state::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub(super) struct GetInput {
    sheet: Option<String>,
    ver: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct PostInput {
    #[serde(alias = "sheet")]
    key: Option<String>,
    #[serde(alias = "version")]
    ver: Option<String>,
    #[serde(default)]
    fields: Vec<FieldInput>,
}

#[derive(Debug, Deserialize)]
struct FieldInput {
    field: String,
    op: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Eq,
    Ne,
}

#[derive(Debug, PartialEq)]
struct Field {
    field: String,
    op: Operator,
    value: Bson,
}

#[derive(Debug, PartialEq)]
struct Request {
    sheet: String,
    version: String,
    fields: Vec<Field>,
}

pub(super) async fn get(
    State(state): State<std::sync::Arc<AppState>>,
    Query(input): Query<GetInput>,
) -> Result<Json<Vec<Document>>, ApiError> {
    let request = parse_get_request(input)?;
    query(&state.game_query, request).await.map(Json)
}

pub(super) async fn post(
    State(state): State<std::sync::Arc<AppState>>,
    Json(input): Json<PostInput>,
) -> Result<Json<Vec<Document>>, ApiError> {
    let request = parse_post_request(input)?;
    query(&state.game_query, request).await.map(Json)
}

async fn query(
    repository: &impl GameQueryRepository,
    request: Request,
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

fn parse_get_request(input: GetInput) -> Result<Request, ApiError> {
    let sheet = required_value(input.sheet, "sheet")?;
    Ok(Request { sheet, version: version_or_default(input.ver), fields: Vec::new() })
}

fn parse_post_request(input: PostInput) -> Result<Request, ApiError> {
    let sheet = required_value(input.key, "key")?;
    let fields = input.fields.into_iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;
    Ok(Request { sheet, version: version_or_default(input.ver), fields })
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

fn parse_field(input: FieldInput) -> Result<Field, ApiError> {
    let field = required_value(Some(input.field), "field")?;
    let op = match input.op.trim().to_ascii_lowercase().as_str() {
        "eq" => Operator::Eq,
        "ne" => Operator::Ne,
        operator => {
            return Err(ApiError::bad_request(format!("unsupported operator: {operator}")));
        }
    };
    let value = to_bson(&input.value)
        .map_err(|error| ApiError::bad_request(format!("invalid filter value: {error}")))?;
    Ok(Field { field, op, value })
}

fn validate_fields(metadata: &GameQuerySheet, fields: &[Field]) -> Result<(), ApiError> {
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

fn build_predicates(fields: &[Field]) -> Document {
    if fields.is_empty() {
        return Document::new();
    }

    let predicates = fields
        .iter()
        .map(|field| {
            let path = format!("data.{}", field.field);
            let predicate = match field.op {
                Operator::Eq => doc! { path: field.value.clone() },
                Operator::Ne => doc! { path: { "$ne": field.value.clone() } },
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
    use crate::db::{GameQueryColumn, GameQueryStoreError};

    struct MockRepository {
        sheet: Option<GameQuerySheet>,
        version_exists: bool,
        rows: Vec<Document>,
        predicates: Mutex<Option<Document>>,
    }

    impl MockRepository {
        fn with_sheet() -> Self {
            Self {
                sheet: Some(GameQuerySheet {
                    version: DEFAULT_VERSION.to_string(),
                    sheet: "alliance_armory_const".to_string(),
                    columns: vec![
                        GameQueryColumn {
                            name: "ID".to_string(),
                            value_type: "integer".to_string(),
                        },
                        GameQueryColumn {
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

    impl GameQueryRepository for MockRepository {
        fn find_sheet<'a>(
            &'a self,
            _version: &'a str,
            _sheet: &'a str,
        ) -> BoxFuture<'a, Result<Option<GameQuerySheet>, GameQueryStoreError>> {
            async move { Ok(self.sheet.clone()) }.boxed()
        }

        fn version_exists<'a>(
            &'a self,
            _version: &'a str,
        ) -> BoxFuture<'a, Result<bool, GameQueryStoreError>> {
            async move { Ok(self.version_exists) }.boxed()
        }

        fn find_rows<'a>(
            &'a self,
            _version: &'a str,
            _sheet: &'a str,
            predicates: Document,
        ) -> BoxFuture<'a, Result<Vec<Document>, GameQueryStoreError>> {
            async move {
                *self.predicates.lock().expect("mutex should not be poisoned") = Some(predicates);
                Ok(self.rows.clone())
            }
            .boxed()
        }
    }

    #[test]
    fn get_request_requires_sheet_and_uses_default_version() {
        let error = parse_get_request(GetInput::default()).expect_err("sheet should be required");
        assert!(matches!(error, ApiError::BadRequest(message) if message == "sheet is required"));

        let request = parse_get_request(GetInput {
            sheet: Some("alliance_armory_const".to_string()),
            ver: None,
        })
        .expect("request should be valid");
        assert_eq!(request.version, DEFAULT_VERSION);
    }

    #[test]
    fn post_request_accepts_eq_and_ne_filters() {
        let request = parse_post_request(PostInput {
            key: Some("alliance_armory_const".to_string()),
            ver: Some("1.2.0".to_string()),
            fields: vec![
                FieldInput {
                    field: "ID".to_string(),
                    op: "eq".to_string(),
                    value: serde_json::json!(1),
                },
                FieldInput {
                    field: "Key".to_string(),
                    op: "NE".to_string(),
                    value: serde_json::json!("DONATE_SCORE_2"),
                },
            ],
        })
        .expect("request should be valid");

        assert_eq!(request.version, "1.2.0");
        assert_eq!(request.fields[0].op, Operator::Eq);
        assert_eq!(request.fields[1].op, Operator::Ne);
    }

    #[test]
    fn post_request_rejects_unknown_operator() {
        let error = parse_post_request(PostInput {
            key: Some("alliance_armory_const".to_string()),
            fields: vec![FieldInput {
                field: "ID".to_string(),
                op: "gt".to_string(),
                value: serde_json::json!(1),
            }],
            ..PostInput::default()
        })
        .expect_err("operator should be rejected");

        assert!(
            matches!(error, ApiError::BadRequest(message) if message == "unsupported operator: gt")
        );
    }

    #[tokio::test]
    async fn query_returns_rows_and_builds_anded_predicates() {
        let repository = MockRepository::with_sheet();
        let request = Request {
            sheet: "alliance_armory_const".to_string(),
            version: DEFAULT_VERSION.to_string(),
            fields: vec![Field {
                field: "ID".to_string(),
                op: Operator::Ne,
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
        let request = Request {
            sheet: "alliance_armory_const".to_string(),
            version: DEFAULT_VERSION.to_string(),
            fields: vec![Field {
                field: "Unknown".to_string(),
                op: Operator::Eq,
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
        let request = Request {
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
        let request = Request {
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
