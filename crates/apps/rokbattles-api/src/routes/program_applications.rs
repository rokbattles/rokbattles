//! Authenticated submission and status endpoints for program applications.
//!
//! The session supplies the user ID. Question text, status, and timestamps are
//! assigned by the server; clients submit only the selected program, its answer,
//! and a Turnstile token.

use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header::CACHE_CONTROL},
    middleware,
    response::IntoResponse,
    routing::{MethodRouter, get, post},
};
use mongodb::{
    bson::{DateTime, oid::ObjectId},
    error::{ErrorKind, WriteFailure},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthenticatedSession,
    db::{FormAnswer, Program, ProgramApplication},
    error::ApiError,
    rate_limit::{self, RateLimitMode, RateLimiter},
    state::AppState,
};

const SUBMISSION_BURST: u32 = 5;
const SUBMISSION_REFILL: Duration = Duration::from_secs(30);
const MAX_SUBMISSION_BYTES: usize = 32 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Submission {
    program: Program,
    answer: String,
    turnstile_token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationSummary {
    id: String,
    program: Program,
    status: String,
    created_at: i64,
}

impl From<ProgramApplication> for ApplicationSummary {
    fn from(application: ProgramApplication) -> Self {
        Self {
            id: application.id.to_hex(),
            program: application.program,
            status: application.status,
            created_at: application.created_at.timestamp_millis(),
        }
    }
}

#[derive(Serialize)]
struct ApplicationsResponse {
    applications: Vec<ApplicationSummary>,
}

/// Builds the status endpoint and the rate-limited submission endpoint.
pub(super) fn routes(mode: RateLimitMode) -> MethodRouter<Arc<AppState>> {
    let limiter = Arc::new(RateLimiter::new(mode, SUBMISSION_BURST, SUBMISSION_REFILL));
    let submit = post(submit)
        .layer(DefaultBodyLimit::max(MAX_SUBMISSION_BYTES))
        .layer(middleware::from_fn_with_state(limiter, rate_limit::enforce));

    get(get_open).merge(submit)
}

/// Lists the authenticated user's open applications.
async fn get_open(
    session: AuthenticatedSession,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let programs = state
        .program_applications
        .as_ref()
        .ok_or_else(|| ApiError::internal("program applications unavailable"))?;

    let applications = programs
        .store
        .list_open(&session.user.discord_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    Ok((
        [(CACHE_CONTROL, "no-store")],
        Json(ApplicationsResponse {
            applications: applications.into_iter().map(Into::into).collect(),
        }),
    ))
}

/// Submits one application for the authenticated user and selected program.
async fn submit(
    session: AuthenticatedSession,
    State(state): State<Arc<AppState>>,
    Json(submission): Json<Submission>,
) -> Result<impl IntoResponse, ApiError> {
    let programs = state
        .program_applications
        .as_ref()
        .ok_or_else(|| ApiError::internal("program applications unavailable"))?;

    let answer = validate_answer(submission.program, &submission.answer)?;

    let role = submission.program.role();

    if session.has_role(role) {
        return Err(ApiError::conflict("program-already-active"));
    }

    let applications = programs
        .store
        .list_open(&session.user.discord_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;

    if applications.iter().any(|application| application.program == submission.program) {
        return Err(ApiError::conflict("open-application-exists"));
    }

    programs.turnstile.verify(&submission.turnstile_token).await?;

    // Verification may take several seconds. Recheck the session and roles while
    // holding the account lock, since either may have changed in the meantime.
    let _operation = state.account_operations.lock(&session.user.discord_id).await;
    session.ensure_active(state.auth_store.as_ref()).await?;

    let user = state
        .auth_store
        .find_user_by_discord_id(&session.user.discord_id)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(ApiError::unauthorized)?;

    if user.roles.contains(role) {
        return Err(ApiError::conflict("program-already-active"));
    }

    let now = DateTime::now();
    let application = ProgramApplication {
        id: ObjectId::new(),
        user_id: session.user.discord_id,
        program: submission.program,
        form: vec![FormAnswer { question: submission.program.question().to_owned(), answer }],
        status: "open".into(),
        approval_started: false,
        created_at: now,
        updated_at: now,
    };

    if let Err(error) = programs.store.insert(&application).await {
        if is_duplicate_key(&error) {
            return Err(ApiError::conflict("open-application-exists"));
        }

        return Err(ApiError::internal(error.to_string()));
    }

    Ok((
        StatusCode::CREATED,
        [(CACHE_CONTROL, "no-store")],
        Json(ApplicationsResponse { applications: vec![application.into()] }),
    ))
}

fn is_duplicate_key(error: &mongodb::error::Error) -> bool {
    matches!(
        error.kind.as_ref(),
        ErrorKind::Write(WriteFailure::WriteError(error)) if error.code == 11000
    )
}

fn validate_answer(program: Program, answer: &str) -> Result<String, ApiError> {
    let answer = answer.trim();

    match program {
        Program::DeveloperProgram => {
            if answer.is_empty() || answer.chars().count() > 5000 {
                return Err(ApiError::bad_request(
                    "Enter your integration plans (up to 5,000 characters).",
                ));
            }

            Ok(answer.to_owned())
        }
        Program::ContentCreator => {
            let invalid = || {
                ApiError::bad_request(
                    "Enter a YouTube channel URL, such as https://www.youtube.com/@yourchannel.",
                )
            };

            if answer.len() > 2048 || answer.chars().any(char::is_whitespace) {
                return Err(invalid());
            }

            let mut url = reqwest::Url::parse(answer).map_err(|_error| invalid())?;

            if url.scheme() != "https"
                || !matches!(
                    url.host_str(),
                    Some("youtube.com" | "www.youtube.com" | "m.youtube.com")
                )
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
            {
                return Err(invalid());
            }

            let path = url.path().trim_end_matches('/');
            let segments: Vec<_> = path.trim_start_matches('/').split('/').collect();
            let valid = match segments.as_slice() {
                [handle] => handle.starts_with('@') && handle.len() > 1,
                ["channel", id] => {
                    id.starts_with("UC")
                        && id.len() == 24
                        && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                }
                ["c" | "user", name] => !name.is_empty(),
                _ => false,
            };

            if !valid {
                return Err(invalid());
            }

            url.set_query(None);
            url.set_fragment(None);

            Ok(url.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_channel_urls_and_removes_tracking_parameters() {
        for url in [
            "https://www.youtube.com/@rokbattles",
            "https://youtube.com/channel/UCabcdefghijklmnopqrstuv",
            "https://m.youtube.com/user/creator",
            "https://youtube.com/c/creator/",
        ] {
            assert!(validate_answer(Program::ContentCreator, url).is_ok(), "{url}");
        }
        assert_eq!(
            validate_answer(
                Program::ContentCreator,
                " https://youtube.com/@creator?si=tracking#about "
            )
            .expect("valid"),
            "https://youtube.com/@creator"
        );
    }

    #[test]
    fn rejects_video_urls_and_spoofed_youtube_hosts() {
        for url in [
            "",
            "https://youtube.com/",
            "http://youtube.com/@creator",
            "https://youtu.be/abc",
            "https://youtube.com/watch?v=abc",
            "https://youtube.com/shorts/abc",
            "https://youtube.com/playlist?list=abc",
            "https://youtube.com.evil.com/@creator",
            "https://youtube.com@evil.com/@creator",
            "https://user@youtube.com/@creator",
            "https://youtube.com:123/@creator",
            "https://youtube.com/@",
            "https://youtube.com/channel/UCshort",
            "https://youtube.com/@bad name",
        ] {
            assert!(validate_answer(Program::ContentCreator, url).is_err(), "{url}");
        }
    }

    #[test]
    fn integration_plan_is_required_trimmed_and_bounded() {
        assert_eq!(
            validate_answer(Program::DeveloperProgram, " Build an alliance dashboard. ")
                .expect("valid"),
            "Build an alliance dashboard."
        );
        for answer in ["".into(), " \n\t".into(), "a".repeat(5001)] {
            validate_answer(Program::DeveloperProgram, &answer).expect_err("invalid plan");
        }
        validate_answer(Program::DeveloperProgram, &"a".repeat(5000))
            .expect("maximum length accepted");
    }

    #[test]
    fn client_cannot_supply_identity_status_questions_or_unknown_programs() {
        for body in [
            r#"{"program":"admin","answer":"x","turnstileToken":"t"}"#,
            r#"{"program":"developer_program","answer":"x","turnstileToken":"t","userId":"other"}"#,
            r#"{"program":"developer_program","answer":"x","turnstileToken":"t","status":"approved"}"#,
            r#"{"program":"developer_program","form":[{"question":"question","answer":"answer"}],"turnstileToken":"t"}"#,
        ] {
            assert!(serde_json::from_str::<Submission>(body).is_err());
        }
    }

    #[test]
    fn rejects_mongodb_operator_objects_in_submission_fields() {
        for program in ["content_creator", "developer_program"] {
            for field in ["program", "answer", "turnstileToken"] {
                for operator in [
                    serde_json::json!({ "$ne": null }),
                    serde_json::json!({ "$gt": "" }),
                    serde_json::json!({ "$regex": ".*" }),
                    serde_json::json!({ "$where": "return true" }),
                ] {
                    let mut body = serde_json::json!({
                        "program": program,
                        "answer": "https://www.youtube.com/@creator",
                        "turnstileToken": "test-token",
                    });
                    body[field] = operator;

                    assert!(
                        serde_json::from_value::<Submission>(body).is_err(),
                        "operator object accepted in {field} for {program}"
                    );
                }
            }
        }
    }

    #[test]
    fn query_shaped_answers_are_serialized_as_bson_strings() {
        for (program, answer, expected) in [
            (Program::DeveloperProgram, r#"{"$ne":null}"#, r#"{"$ne":null}"#),
            (
                Program::DeveloperProgram,
                r#"{"$where":"function() { return true; }"}"#,
                r#"{"$where":"function() { return true; }"}"#,
            ),
            (
                Program::DeveloperProgram,
                r#"{"$set":{"status":"approved","userId":"other"}}"#,
                r#"{"$set":{"status":"approved","userId":"other"}}"#,
            ),
            (
                Program::ContentCreator,
                "https://www.youtube.com/@creator?filter=%7B%22%24ne%22%3Anull%7D",
                "https://www.youtube.com/@creator",
            ),
        ] {
            let submission: Submission = serde_json::from_value(serde_json::json!({
                "program": program,
                "answer": answer,
                "turnstileToken": "test-token",
            }))
            .expect("string answer");
            let answer =
                validate_answer(submission.program, &submission.answer).expect("valid answer");
            let application = ProgramApplication {
                id: ObjectId::new(),
                user_id: "authenticated-user".to_owned(),
                program: submission.program,
                form: vec![FormAnswer {
                    question: submission.program.question().to_owned(),
                    answer,
                }],
                status: "open".to_owned(),
                approval_started: false,
                created_at: DateTime::from_millis(0),
                updated_at: DateTime::from_millis(0),
            };

            let document = mongodb::bson::to_document(&application).expect("application BSON");
            let form = document.get_array("form").expect("form");
            let entry = form[0].as_document().expect("form entry");

            assert_eq!(entry.get_str("answer").expect("BSON string"), expected);
            assert_eq!(entry.len(), 2);
            assert_eq!(document.get_str("userId").expect("owner"), "authenticated-user");
            assert_eq!(document.get_str("status").expect("status"), "open");
        }
    }
}
