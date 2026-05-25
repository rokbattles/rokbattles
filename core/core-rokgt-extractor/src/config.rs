use std::{num::NonZeroUsize, time::Duration};

pub(crate) const CLIENT_ID: &str = "rok_game_tools_lglo";
pub(crate) const GAME_TOOLS_APP_ID: u64 = 2_104_267;
pub(crate) const DEFAULT_LANGUAGE: &str = "en_US";
pub(crate) const PASSPORT_API: &str = "https://passport-global-api.lilithgame.com";
pub(crate) const GAME_TOOLS_API: &str = "https://rok-game-tools-global-api.lilith.com";
pub(crate) const PLATFORM_API: &str = "https://plat-rok-gametools-global-api.lilithgames.com";
pub(crate) const PASSPORT_ORIGIN: &str = "https://passport-global.lilith.com";
pub(crate) const GAME_TOOLS_ORIGIN: &str = "https://rok-game-tools-global.lilith.com";

pub(crate) const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/145.0.0.0 Safari/537.36";

pub(crate) const DEFAULT_BATCH_SIZE: usize = 64;

pub(crate) fn default_batch_size() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_BATCH_SIZE).unwrap_or(NonZeroUsize::MIN)
}

/// Runtime settings for [`crate::RokGtClient`].
#[derive(Debug, Clone)]
pub struct RokGtConfig {
    /// Passport signing access key.
    pub access_key: String,
    /// Passport signing secret key.
    pub secret_key: String,
    /// Request timeout.
    pub timeout: Duration,
}

impl RokGtConfig {
    /// Create config with the required Passport signing keys.
    pub fn new(access_key: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            timeout: Duration::from_secs(30),
        }
    }
}
