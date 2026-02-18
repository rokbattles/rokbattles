use std::time::Duration;

/// Default base URL for the ROK Game Tools Global API.
pub const DEFAULT_BASE_URL: &str = "https://rok-game-tools-global-api.lilith.com";
/// Default base URL for the platform ROK Game Tools API host.
pub const DEFAULT_PLATFORM_BASE_URL: &str = "https://plat-rok-gametools-global-api.lilithgames.com";

/// Runtime configuration used to construct [`crate::RokGtClient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokGtConfig {
    /// Base API URL.
    pub base_url: String,
    /// Base API URL used by platform-hosted endpoints.
    pub platform_base_url: String,
    /// Token for the `Pauthorization` header.
    pub p_authorization_token: String,
    /// Token for the `Bauthorization` header.
    pub b_authorization_token: String,
    /// Value for the `Lang` header.
    pub lang: String,
    /// Request timeout applied to all HTTP calls.
    pub timeout: Duration,
}

impl RokGtConfig {
    /// Create a config with required auth tokens and sensible defaults.
    pub fn new(
        p_authorization_token: impl Into<String>,
        b_authorization_token: impl Into<String>,
    ) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            platform_base_url: DEFAULT_PLATFORM_BASE_URL.to_string(),
            p_authorization_token: p_authorization_token.into(),
            b_authorization_token: b_authorization_token.into(),
            lang: "en_US".to_string(),
            timeout: Duration::from_secs(20),
        }
    }

    /// Override the API base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Override the platform API base URL.
    pub fn with_platform_base_url(mut self, platform_base_url: impl Into<String>) -> Self {
        self.platform_base_url = platform_base_url.into();
        self
    }

    /// Override the default language header value.
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = lang.into();
        self
    }

    /// Override request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_match_expected_values() {
        let config = RokGtConfig::new("p-token", "b-token");
        assert_eq!(config.base_url, DEFAULT_BASE_URL);
        assert_eq!(config.platform_base_url, DEFAULT_PLATFORM_BASE_URL);
        assert_eq!(config.lang, "en_US");
        assert_eq!(config.timeout, Duration::from_secs(20));
    }
}
