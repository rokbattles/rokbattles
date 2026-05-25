#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Extracts kingdom member data from the ROK Game Tools APIs.

mod auth;
mod batcher;
mod client;
mod config;
mod date;
mod error;
mod models;
mod parse;
mod util;

pub use auth::Credentials;
pub use batcher::KingdomMemberBatcher;
pub use client::RokGtClient;
pub use config::RokGtConfig;
pub use error::RokGtError;
pub use models::{KingdomMember, KingdomMemberBatch, group_members_by_kingdom};
