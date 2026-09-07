//! Borrows the required `body.content` object and shares SDK field readers.
//!
//! The root, body, and content must each be objects. Missing child keys and
//! present children of the wrong type retain the SDK's distinct errors.

pub(crate) use rokbattles_mail_sdk::{
    ExtractError, optional_number_field_or_zero, require_child_object, require_number_field,
    require_object,
};
use serde_json::{Map, Value};

/// Returns the nested `body.content` object from an RSS mail payload.
pub(crate) fn require_content(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "content")
}
