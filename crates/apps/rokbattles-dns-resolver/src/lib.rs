//! DNS-over-HTTPS resolution for the TCP gateway fleet.
//!
//! The iOS endpoint is non-recursive and only answers for
//! `rocgate.lilithgame.com`. The separate Intra endpoint answers that hostname
//! locally and forwards other queries to Cloudflare DNS-over-HTTPS.

#![forbid(unsafe_code)]

pub(crate) const DNS_MEDIA_TYPE: &str = "application/dns-message";
pub(crate) const MAX_DNS_MESSAGE_BYTES: usize = 65_535;

mod config;
mod forwarder;
mod http;
mod resolver;

pub use config::{Config, ConfigError};
pub use forwarder::{
    CLOUDFLARE_DOH_FALLBACK_URL, CLOUDFLARE_DOH_PRIMARY_URL, DoHForwarder, ForwardError,
    MAX_CONCURRENT_UPSTREAM_QUERIES,
};
pub use http::router;
pub use resolver::{ROCGATE_HOSTNAME, ResolveError, Resolver, ResolverConfigError};
