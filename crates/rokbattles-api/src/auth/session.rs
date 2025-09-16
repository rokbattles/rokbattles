use std::future::Future;
use std::time::SystemTime;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, header, request::Parts},
};
use blake3::hash;
use mongodb::bson::{DateTime, Document, doc};

use crate::AppState;

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: String,
}

impl<S> FromRequestParts<S> for SessionUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        let st = AppState::from_ref(state);
        let session_cookie = parts
            .headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| extract_cookie(value, &st.session_cookie_name));

        async move {
            let session_id = match session_cookie {
                Some(value) if !value.is_empty() => value,
                _ => return Err((StatusCode::UNAUTHORIZED, "unauthorized")),
            };

            let session_hash = hash(session_id.as_bytes()).to_hex().to_string();
            let now = SystemTime::now();
            let now_dt = DateTime::from_system_time(now);

            let sessions = st.db.collection::<Document>("sessions");
            let filter = doc! {
                "sessionIdHash": &session_hash,
                "expiresAt": { "$gt": &now_dt },
            };

            let session_doc = sessions
                .find_one(filter)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "service error"))?
                .ok_or((StatusCode::UNAUTHORIZED, "unauthorized"))?;

            let user_id = session_doc
                .get_str("userId")
                .map(|value| value.to_string())
                .map_err(|_| (StatusCode::UNAUTHORIZED, "unauthorized"))?;

            Ok(SessionUser { id: user_id })
        }
    }
}

fn extract_cookie(header_value: &str, name: &str) -> Option<String> {
    header_value
        .split(';')
        .map(|part| part.trim())
        .find_map(|part| {
            let mut pieces = part.splitn(2, '=');
            let key = pieces.next()?.trim();
            if key != name {
                return None;
            }
            let value = pieces.next()?.trim();
            Some(value.to_string())
        })
}
