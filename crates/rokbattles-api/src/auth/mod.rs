mod discord;
pub mod session;

use crate::AppState;
use axum::{Router, routing::get};

pub fn router() -> Router<AppState> {
    Router::new().route("/discord/callback", get(discord::callback))
}
