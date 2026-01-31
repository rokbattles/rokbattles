use crate::config::Config;
use crate::storage::Storage;

/// Shared application state for request handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub storage: Storage,
}
