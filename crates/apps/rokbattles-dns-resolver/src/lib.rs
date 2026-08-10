//! DNS-over-HTTPS resolution for a TCP relay.
//!
//! The iOS endpoint is non-recursive and only answers for one configured
//! hostname. The separate Intra endpoint answers that hostname locally and
//! forwards other queries to a configured upstream DoH resolver.

#![forbid(unsafe_code)]

pub(crate) const DNS_MEDIA_TYPE: &str = "application/dns-message";
pub(crate) const MAX_DNS_MESSAGE_BYTES: usize = 65_535;

mod config;
mod forwarder;
mod http;
mod resolver;

pub use config::{Config, ConfigError};
pub use forwarder::{DoHForwarder, ForwardError};
pub use http::router;
pub use resolver::{ResolveError, Resolver};
