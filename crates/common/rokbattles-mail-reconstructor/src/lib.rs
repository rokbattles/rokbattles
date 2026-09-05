//! Reconstructs persistent mail files from network protobuf mail entries.
//!
//! Load a runtime protocol artifact with [`MailReconstructor::load`], then reuse
//! the reconstructor for individual `MailEntity` payloads. Reconstruction decodes
//! the envelope, inflates compressed bodies, adapts each supported mail format,
//! and passes the resulting JSON value to [`rokbattles_mail_encoder::encode`].
//! The returned bytes can be stored as `Persistent.Mail.<id>`; this crate does
//! not write files or parse network framing.
//!
//! # Supported bodies
//!
//! Input type names are case-sensitive. `Battle2` is normalized to `Battle`.
//!
//! | Network type | Body decoding and persistent shape |
//! | --- | --- |
//! | `Battle`, `Battle2` | JSON and split attack records become `body.content`. |
//! | `DuelBattle2` | `DuelMailReport` protobuf becomes `body.detail`. |
//! | `Rss` | `MailRss` protobuf becomes `body.content`. |
//! | `BarCanyonKillBoss` | `EliteBarReportInfo` fields become `body.content`. |
//! | `EventMemberLootReport` | `EventMemeberLootInfo` fields become `body.content`. |
//! | `System` | `MailSys` fields become subtype fields and generated reward text. |
//! | `Alliance` | `MailSys` fields become `type`, `param`, and decoded `kvs`. |
//!
//! The assembled mail must also be recognized by
//! [`rokbattles_mail_registry::detect_mail_type`]. In particular, System and
//! Alliance mail need supported subtypes, and event member loot mail needs the
//! GVE event label. Registry recognition does not run the mail processors or
//! validate all fields they require.
//!
//! # Reconstruction behavior
//!
//! This recreates the persistent representation from available network fields.
//! Missing envelope values generally become empty strings or zeroes. A missing
//! or zero entry server ID uses [`ReconstructionContext::server_id`]. Some body
//! adapters supply display text and defaults that are absent from the network
//! message; they do not recover the original client's localized text.
//!
//! Artifacts are limited to 32 MiB. Entries and declared inflated body lengths
//! are limited to 25 MiB. Output file size has no separate bound here.
//!
//! # Contributor guide
//!
//! `artifact` resolves field numbers and indexes message descriptors; `protobuf`
//! reads wire fields; `entity` decodes the mail envelope. `dynamic` converts
//! descriptor-backed bodies to JSON, while `body` applies mail-specific shapes
//! and `value` handles compression and Lua table conventions. `reconstructor`
//! assembles the persistent fields, checks the category, and encodes the file.

#![forbid(unsafe_code)]

mod artifact;
mod body;
mod dynamic;
mod entity;
mod error;
mod protobuf;
mod reconstructor;
mod value;

pub use error::ReconstructionError;
pub use reconstructor::{MailReconstructor, ReconstructedMail, ReconstructionContext};
