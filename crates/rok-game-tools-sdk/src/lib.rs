#![forbid(unsafe_code)]

//! SDK for the ROK Game Tools Global API.
//!
//! This crate provides a typed async client with shared configuration and
//! domain models for endpoints used by ROKBattles services.

mod client;
mod config;
mod error;
pub mod models;

pub use client::RokGtClient;
pub use config::RokGtConfig;
pub use error::RokGtError;
