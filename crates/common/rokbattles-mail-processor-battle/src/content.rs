//! Borrows the required `body.content` object and shares SDK field readers.
//!
//! Each extractor reads the same decoded input independently. These helpers do
//! not detect the mail type or unwrap arrays around the root object.

pub(crate) use rokbattles_mail_sdk::{
    ExtractError, require_child_object, require_object, require_string_field, require_u64_field,
};
use serde_json::{Map, Value};

/// Returns the nested Battle content object.
pub(crate) fn require_content(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "content")
}
