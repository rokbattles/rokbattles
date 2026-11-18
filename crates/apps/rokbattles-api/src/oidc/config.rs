//! Operator settings and signing keys for the private OIDC issuer.

use std::env;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Url;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::Serialize;

use super::hash;

pub(super) const ALGORITHM: &str = "Ed25519";

/// An enabled issuer and its signing keys.
pub struct OidcConfig {
    pub(super) issuer: String,
    signer: Ed25519KeyPair,
    keys: Vec<PublicKey>,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid OIDC configuration: {0}")]
pub struct ConfigError(&'static str);

#[derive(Serialize)]
struct PublicKey {
    kty: &'static str,
    crv: &'static str,
    alg: &'static str,
    #[serde(rename = "use")]
    usage: &'static str,
    kid: String,
    x: String,
}

impl PublicKey {
    fn new(bytes: &[u8]) -> Self {
        let x = URL_SAFE_NO_PAD.encode(bytes);
        Self { kty: "OKP", crv: ALGORITHM, alg: ALGORITHM, usage: "sig", kid: hash(&x), x }
    }
}

impl OidcConfig {
    /// Reads issuer configuration. An empty issuer leaves the provider disabled.
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        let Some(issuer) = env::var("OIDC_ISSUER").ok().filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let seed = env::var("OIDC_SIGNING_SEED")
            .map_err(|_error| ConfigError("OIDC_SIGNING_SEED is required"))?;
        let verify_keys = env::var("OIDC_PUBLIC_KEYS").unwrap_or_default();

        Self::new(&issuer, &seed, &verify_keys).map(Some)
    }

    pub(super) fn new(issuer: &str, seed: &str, verify_keys: &str) -> Result<Self, ConfigError> {
        let url =
            Url::parse(issuer).map_err(|_error| ConfigError("OIDC_ISSUER must be an origin"))?;
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if url.origin().ascii_serialization() != issuer
            || !(url.scheme() == "https" || url.scheme() == "http" && loopback)
        {
            return Err(ConfigError("OIDC_ISSUER must be an HTTPS origin or HTTP loopback origin"));
        }
        let seed = URL_SAFE_NO_PAD.decode(seed).map_err(|_error| {
            ConfigError("OIDC_SIGNING_SEED must be 32 base64url-encoded bytes")
        })?;
        let signer = Ed25519KeyPair::from_seed_unchecked(&seed).map_err(|_error| {
            ConfigError("OIDC_SIGNING_SEED must be 32 base64url-encoded bytes")
        })?;
        let mut keys = vec![PublicKey::new(signer.public_key().as_ref())];

        for encoded in verify_keys.split(',').map(str::trim).filter(|value| !value.is_empty()) {
            let bytes = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_error| ConfigError("OIDC_PUBLIC_KEYS contains an invalid public key"))?;
            if bytes.len() != 32 {
                return Err(ConfigError("OIDC_PUBLIC_KEYS contains an invalid public key"));
            }
            let key = PublicKey::new(&bytes);
            if keys.iter().any(|other| other.kid == key.kid) {
                return Err(ConfigError("OIDC_PUBLIC_KEYS contains a duplicate signing key"));
            }
            keys.push(key);
        }

        Ok(Self { issuer: issuer.to_owned(), signer, keys })
    }

    pub(super) fn sign(&self, claims: &impl Serialize) -> Result<String, serde_json::Error> {
        let header = serde_json::json!({
            "alg": ALGORITHM,
            "kid": self.keys.first().map(|key| &key.kid),
            "typ": "JWT"
        });
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims)?);
        let message = format!("{header}.{claims}");
        let signature = URL_SAFE_NO_PAD.encode(self.signer.sign(message.as_bytes()));
        Ok(format!("{message}.{signature}"))
    }

    pub(super) fn jwks(&self) -> impl Serialize + '_ {
        #[derive(Serialize)]
        struct Jwks<'a> {
            keys: &'a [PublicKey],
        }

        Jwks { keys: &self.keys }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publishes_current_and_rollover_keys_without_private_parameters() {
        let seed = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        let next_seed = [9_u8; 32];
        let next = Ed25519KeyPair::from_seed_unchecked(&next_seed).expect("next signer");
        let next_x = URL_SAFE_NO_PAD.encode(next.public_key().as_ref());
        let config = OidcConfig::new("https://example.test", &seed, &next_x).expect("key set");
        let value = serde_json::to_value(config.jwks()).expect("JWKS");
        let keys = value.get("keys").and_then(serde_json::Value::as_array).expect("keys");
        assert_eq!(keys.len(), 2);
        assert!(keys.iter().all(|key| key.get("d").is_none() && key["alg"] == "Ed25519"));
        assert!(keys.iter().any(|key| key["x"] == next_x));
    }

    #[test]
    fn rejects_insecure_non_loopback_issuers() {
        let seed = URL_SAFE_NO_PAD.encode([7_u8; 32]);
        assert!(OidcConfig::new("http://public.example", &seed, "").is_err());
    }
}
