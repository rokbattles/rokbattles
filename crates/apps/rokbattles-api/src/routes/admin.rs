//! Administrator endpoints for reviewing program applications.
//!
//! Approval grants the role associated with the application. Denial closes the
//! application so the account can submit a new one.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, header::CACHE_CONTROL},
    response::IntoResponse,
    routing::get,
};
use futures::future::try_join_all;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthenticatedSession, RequireRole, Role, roles::Admin},
    db::{FormAnswer, Program, ProgramApplication, ProgramApplicationsStore, ReportsStore},
    error::ApiError,
    routes::governor::bind::load_claimed_governors,
    state::AppState,
};

const PAGE_SIZE: u32 = 10;
const NO_STORE: [(axum::http::HeaderName, &str); 1] = [(CACHE_CONTROL, "no-store")];

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationPage {
    applications: Vec<ApplicationListItem>,
    has_next_page: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationListItem {
    #[serde(flatten)]
    application: ApplicationSummary,
    default_governor: Option<DefaultGovernor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationSummary {
    id: String,
    discord_id: String,
    program: Program,
    status: String,
    created_at: i64,
}

impl From<&ProgramApplication> for ApplicationSummary {
    fn from(application: &ProgramApplication) -> Self {
        let status = if application.status == "open" && application.approval_started {
            "approving"
        } else {
            &application.status
        };

        Self {
            id: application.id.to_hex(),
            discord_id: application.user_id.clone(),
            program: application.program,
            status: status.to_owned(),
            created_at: application.created_at.timestamp_millis(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationDetail {
    #[serde(flatten)]
    application: ApplicationSummary,
    form: Vec<FormAnswer>,
    email: Option<String>,
    default_governor: Option<DefaultGovernor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultGovernor {
    governor_id: i64,
    governor_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    decision: Decision,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Decision {
    Approve,
    Deny,
}

impl Decision {
    fn final_status(&self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Deny => "closed",
        }
    }
}

pub(super) fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/program-applications", get(list))
        .route("/program-applications/{id}", get(detail).patch(review))
        .layer(DefaultBodyLimit::max(1024))
}

async fn list(
    _admin: RequireRole<Admin>,
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, ApiError> {
    let page = pagination.page.unwrap_or(1);
    let offset = page.checked_sub(1).ok_or_else(|| ApiError::bad_request("Invalid page"))?;
    let offset = u64::from(offset) * u64::from(PAGE_SIZE);
    let store = application_store(&state)?;

    let mut applications =
        store.list_for_review(offset, i64::from(PAGE_SIZE + 1)).await.map_err(database_error)?;

    let has_next_page = applications.len() > PAGE_SIZE as usize;
    applications.truncate(PAGE_SIZE as usize);

    let applications = try_join_all(applications.iter().map(|application| async {
        let default_governor = default_governor(&state.reports_store, &application.user_id).await?;

        Ok::<_, ApiError>(ApplicationListItem { application: application.into(), default_governor })
    }))
    .await?;

    Ok((NO_STORE, Json(ApplicationPage { applications, has_next_page })))
}

async fn detail(
    _admin: RequireRole<Admin>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let application = find_application(application_store(&state)?, parse_id(&id)?).await?;

    let user = async {
        state.auth_store.find_user_by_discord_id(&application.user_id).await.map_err(database_error)
    };
    let governor = default_governor(&state.reports_store, &application.user_id);
    let (user, default_governor) = tokio::try_join!(user, governor)?;

    Ok((
        NO_STORE,
        Json(ApplicationDetail {
            application: (&application).into(),
            form: application.form,
            email: user.map(|user| user.email),
            default_governor,
        }),
    ))
}

async fn review(
    _admin: RequireRole<Admin>,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(review): Json<Review>,
) -> Result<impl IntoResponse, ApiError> {
    let id = parse_id(&id)?;
    let store = application_store(&state)?;
    let application = find_application(store, id).await?;

    // Order this review with submissions and account deletion in this process.
    let _operation = state.account_operations.lock(&application.user_id).await;

    // The reviewer's session or role may have changed while waiting for the lock.
    let admin = AuthenticatedSession::authenticate(&headers, state.auth_store.as_ref()).await?;
    admin.require_role(Role::Admin)?;

    let result = match review.decision {
        Decision::Deny => store.deny(id).await.map_err(database_error)?,
        Decision::Approve if application.status == "open" => {
            approve(&state, store, &application).await?
        }
        Decision::Approve => None,
    };

    let application = match result {
        Some(application) => application,
        None => find_application(store, id).await?,
    };

    if application.status != review.decision.final_status() {
        return Err(ApiError::conflict("application-already-reviewed"));
    }

    Ok((NO_STORE, Json(ApplicationSummary::from(&application))))
}

async fn approve(
    state: &AppState,
    store: &ProgramApplicationsStore,
    application: &ProgramApplication,
) -> Result<Option<ProgramApplication>, ApiError> {
    // Leave the application eligible for denial if the applicant no longer exists.
    if state
        .auth_store
        .find_user_by_discord_id(&application.user_id)
        .await
        .map_err(database_error)?
        .is_none()
    {
        return Err(ApiError::not_found("Applicant account not found"));
    }

    if !store.start_approval(application.id).await.map_err(database_error)? {
        return Ok(None);
    }

    if !store.grant_role(application).await.map_err(database_error)? {
        return Err(ApiError::not_found("Applicant account not found"));
    }

    store.complete_approval(application.id).await.map_err(database_error)
}

async fn default_governor(
    store: &ReportsStore,
    user_id: &str,
) -> Result<Option<DefaultGovernor>, ApiError> {
    let governors = load_claimed_governors(store, user_id, None).await?;
    let governor = governors.into_iter().find(|governor| governor.default);

    Ok(governor.map(|governor| DefaultGovernor {
        governor_id: governor.governor_id,
        governor_name: governor.governor_name,
    }))
}

fn application_store(state: &AppState) -> Result<&ProgramApplicationsStore, ApiError> {
    state
        .program_applications
        .as_ref()
        .map(|programs| &programs.store)
        .ok_or_else(|| ApiError::internal("program applications unavailable"))
}

fn parse_id(id: &str) -> Result<ObjectId, ApiError> {
    ObjectId::parse_str(id).map_err(|_error| ApiError::bad_request("Invalid application ID"))
}

async fn find_application(
    store: &ProgramApplicationsStore,
    id: ObjectId,
) -> Result<ProgramApplication, ApiError> {
    store
        .find_by_id(id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| ApiError::not_found("Application not found"))
}

fn database_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use mongodb::bson::{DateTime, doc, from_document};

    use super::*;

    #[test]
    fn admin_summary_omits_form_and_exposes_unfinished_approvals() {
        let mut document = doc! {
            "_id": ObjectId::new(),
            "userId": "123",
            "program": "developer_program",
            "status": "open",
            "createdAt": DateTime::from_millis(100),
            "updatedAt": DateTime::from_millis(100),
        };
        let legacy: ProgramApplication =
            from_document(document.clone()).expect("legacy application");
        let summary = serde_json::to_value(ApplicationSummary::from(&legacy)).expect("summary");

        assert_eq!(summary["status"], "open");
        assert!(summary.get("form").is_none());

        document.insert("approvalStarted", true);
        let in_progress: ProgramApplication = from_document(document).expect("reserved approval");

        assert_eq!(ApplicationSummary::from(&in_progress).status, "approving");
    }

    #[test]
    fn review_payload_cannot_override_the_applicant_or_granted_role() {
        for payload in [
            r#"{"decision":"approve","discordId":"123"}"#,
            r#"{"decision":"approve","role":"admin"}"#,
            r#"{"decision":"admin"}"#,
        ] {
            assert!(serde_json::from_str::<Review>(payload).is_err());
        }
    }
}
