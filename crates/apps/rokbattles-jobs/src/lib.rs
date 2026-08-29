#![forbid(unsafe_code)]

//! Shared code for the jobs.

pub(crate) mod commander_catalog;
pub mod config;
pub mod error;
pub mod precompute_barbarian;
pub mod precompute_barbarianfort;
pub mod precompute_baulur;
pub mod precompute_cmdr_pairings_v2;
pub mod precompute_drastc;
pub mod precompute_kahar_treasure;
pub mod precompute_karuak_ceremony;
pub mod refresh_binds;
pub mod scheduler;
