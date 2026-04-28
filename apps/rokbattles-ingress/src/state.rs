use crate::{config::Config, storage::Storage};

/// Shared application state for request handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub storage: Storage,
}
