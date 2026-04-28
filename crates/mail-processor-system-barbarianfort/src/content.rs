//! Helpers for walking SystemBarbarianFort mail content.

pub(crate) use mail_processor_sdk::{
    ExtractError, require_child_object, require_number_field, require_object, require_string_field,
    require_u64_field,
};
use serde_json::{Map, Value};

/// Returns the nested `body` object from the root payload.
pub(crate) fn require_body(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    require_child_object(root, "body")
}
