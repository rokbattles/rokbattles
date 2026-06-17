#![forbid(unsafe_code)]

//! Decoder for Rise of Kingdoms `Persistent.Mail.*` files.

mod cursor;
mod error;
mod format;
mod number;
mod value;

pub use cursor::decode;
pub use error::DecodeError;
