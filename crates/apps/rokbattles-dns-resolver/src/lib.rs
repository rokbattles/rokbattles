//! DNS-over-HTTPS resolution for a TCP relay.
//!
//! This crate intentionally has no upstream DNS resolver. It only returns
//! configured A and optional AAAA records for one configured hostname; all
//! other names receive `REFUSED`.

#![forbid(unsafe_code)]

mod config;
mod http;
mod resolver;

pub use config::{Config, ConfigError};
pub use http::router;
pub use resolver::{ResolveError, Resolver};
