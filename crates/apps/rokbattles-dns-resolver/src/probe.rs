//! Tracks pending browser DNS checks in memory.

use std::{env, sync::Arc, time::Duration};

use axum::http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION};
use rustc_hash::FxHashMap;
use subtle::ConstantTimeEq as _;
use tokio::{
    sync::{Mutex, watch},
    time::{Instant, interval, timeout_at},
};

const DOMAIN: &str = "dns-probe.rokbattles.com";
const CHECK_LIFETIME: Duration = Duration::from_secs(10);
const MAX_PENDING: usize = 256;

pub(crate) const NONCE_LENGTH: usize = 32;
pub(crate) type Nonce = [u8; NONCE_LENGTH];

struct PendingCheck {
    expires_at: Instant,
    observed: watch::Sender<bool>,
}

/// Stores pending DNS checks and records matching lookups.
#[derive(Clone)]
pub struct DnsProbe(Arc<ProbeState>);

struct ProbeState {
    authorization: HeaderValue,
    pending: Mutex<FxHashMap<Nonce, PendingCheck>>,
}

/// An invalid value for `DNS_PROBE_SECRET`.
#[derive(Debug, thiserror::Error)]
#[error("DNS_PROBE_SECRET must be a 64-character hex string when configured")]
pub struct ProbeConfigError;

impl DnsProbe {
    /// Creates a probe from `DNS_PROBE_SECRET`.
    ///
    /// Returns `Ok(None)` if the variable is unset, empty, or invalid Unicode.
    ///
    /// # Errors
    ///
    /// Returns an error if the secret is not a 64-character hexadecimal string.
    ///
    /// # Panics
    ///
    /// Panics if a valid secret is set and there is no active Tokio runtime.
    pub fn from_env() -> Result<Option<Self>, ProbeConfigError> {
        let secret = env::var("DNS_PROBE_SECRET").unwrap_or_default();
        if secret.is_empty() {
            return Ok(None);
        }

        Self::new(&secret).map(Some)
    }

    pub(crate) fn new(secret: &str) -> Result<Self, ProbeConfigError> {
        if secret.len() != 64 || !secret.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ProbeConfigError);
        }

        let mut authorization = HeaderValue::from_str(&format!("Bearer {secret}"))
            .map_err(|_error| ProbeConfigError)?;
        authorization.set_sensitive(true);

        let state = Arc::new(ProbeState { authorization, pending: Mutex::default() });
        // Let the expiry task exit when the router drops the probe state.
        let weak_state = Arc::downgrade(&state);
        tokio::spawn(async move {
            let mut expiry_tick = interval(Duration::from_secs(1));
            loop {
                expiry_tick.tick().await;
                let Some(state) = weak_state.upgrade() else {
                    break;
                };

                let now = Instant::now();
                state.pending.lock().await.retain(|_, check| check.expires_at > now);
            }
        });

        Ok(Self(state))
    }

    pub(crate) fn authenticate(
        &self,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Nonce, StatusCode> {
        if !headers.get(AUTHORIZATION).is_some_and(|value| {
            bool::from(value.as_bytes().ct_eq(self.0.authorization.as_bytes()))
        }) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        parse_nonce(body).ok_or(StatusCode::BAD_REQUEST)
    }

    pub(crate) async fn observe(&self, nonce: Nonce) {
        if let Some(check) = self.0.pending.lock().await.get(&nonce)
            && check.expires_at > Instant::now()
        {
            // Keep the observation if the result request has yet to subscribe.
            check.observed.send_replace(true);
        }
    }

    pub(crate) async fn start(&self, nonce: Nonce) -> StatusCode {
        let mut pending = self.0.pending.lock().await;
        let now = Instant::now();
        pending.retain(|_, check| check.expires_at > now);

        if pending.contains_key(&nonce) {
            return StatusCode::CONFLICT;
        }
        if pending.len() >= MAX_PENDING {
            return StatusCode::SERVICE_UNAVAILABLE;
        }

        let observed = watch::Sender::new(false);
        pending.insert(nonce, PendingCheck { expires_at: now + CHECK_LIFETIME, observed });
        StatusCode::NO_CONTENT
    }

    pub(crate) async fn result(&self, nonce: Nonce) -> StatusCode {
        let (mut observed, expires_at) = {
            let pending = self.0.pending.lock().await;
            let Some(check) = pending.get(&nonce) else {
                return StatusCode::NOT_FOUND;
            };

            if check.observed.receiver_count() != 0 {
                return StatusCode::CONFLICT;
            }

            (check.observed.subscribe(), check.expires_at)
        };

        let confirmed = expires_at > Instant::now()
            && matches!(
                timeout_at(expires_at, observed.wait_for(|observed| *observed)).await,
                Ok(Ok(_))
            );

        let mut pending = self.0.pending.lock().await;
        // An expired nonce can be registered again while this request waits.
        let same_check = pending.get(&nonce).is_some_and(|check| check.expires_at == expires_at);
        if same_check {
            pending.remove(&nonce);
        }

        if confirmed && same_check { StatusCode::NO_CONTENT } else { StatusCode::NOT_FOUND }
    }
}

pub(crate) fn hostname_nonce(hostname: &str) -> Option<Nonce> {
    let hostname = hostname.strip_suffix('.').unwrap_or(hostname);
    let (label, domain) = hostname.split_once('.')?;

    if !domain.eq_ignore_ascii_case(DOMAIN) {
        return None;
    }

    parse_nonce(label.as_bytes())
}

fn parse_nonce(bytes: &[u8]) -> Option<Nonce> {
    let mut nonce = Nonce::try_from(bytes).ok()?;
    if !nonce.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }

    nonce.make_ascii_lowercase();
    Some(nonce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn attempts_should_ignore_early_and_unrelated_observations_and_expire() {
        let probe = DnsProbe::new(&"0".repeat(64)).expect("probe should build");
        let nonce = [b'a'; 32];

        probe.observe(nonce).await;
        assert_eq!(probe.start(nonce).await, StatusCode::NO_CONTENT);
        probe.observe([b'b'; 32]).await;

        {
            let mut pending = probe.0.pending.lock().await;
            let check = pending.get_mut(&nonce).expect("attempt should be pending");
            assert!(!*check.observed.borrow());
            check.expires_at = Instant::now();
        }

        probe.observe(nonce).await;
        assert_eq!(probe.result(nonce).await, StatusCode::NOT_FOUND);
        assert!(probe.0.pending.lock().await.is_empty());
    }
}
