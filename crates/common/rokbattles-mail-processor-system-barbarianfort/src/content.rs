//! Borrows the required root `body` object and shares SDK field readers.
//!
//! The body is structured data, while its optional `content` member is localized
//! text interpreted by the body extractor.

pub(crate) use rokbattles_mail_sdk::{
    ExtractError, require_child_object, require_number_field, require_object, require_string_field,
    require_u64_field,
};
use serde_json::{Map, Value};

/// Returns the nested `body` object from the root payload.
pub(crate) fn require_body(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    require_child_object(root, "body")
}
