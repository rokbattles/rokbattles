//! DNS-over-HTTPS service for the ROK Battles gateway fleet.
//!
//! Both `/query` and `/intra` answer queries for [`ROCGATE_HOSTNAMES`] using the
//! same configured gateways. `/intra` forwards unrelated queries to Cloudflare.
//! `/query` returns `REFUSED` for those queries.
//!
//! Both endpoints handle browser probe names locally and return `NXDOMAIN`.
//! When probes are enabled, a matching lookup confirms a pending check.

#![forbid(unsafe_code)]

pub(crate) const DNS_MEDIA_TYPE: &str = "application/dns-message";
pub(crate) const MAX_DNS_MESSAGE_BYTES: usize = 65_535;

mod config;
mod forwarder;
mod http;
mod probe;
mod resolver;

pub use config::{Config, ConfigError};
pub use forwarder::{
    CLOUDFLARE_DOH_FALLBACK_URL, CLOUDFLARE_DOH_PRIMARY_URL, DoHForwarder, ForwardError,
    MAX_CONCURRENT_UPSTREAM_QUERIES,
};
pub use http::{router, router_with_probe};
pub use probe::{DnsProbe, ProbeConfigError};
pub use resolver::{ROCGATE_HOSTNAMES, ResolveError, Resolver, ResolverConfigError};
