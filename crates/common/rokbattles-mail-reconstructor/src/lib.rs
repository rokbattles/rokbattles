//! Runtime reconstruction of network mail entries into `Persistent.Mail` file bytes.

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
