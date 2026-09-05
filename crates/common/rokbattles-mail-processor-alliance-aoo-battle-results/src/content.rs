//! Borrows the required `body.kvs` object and shares SDK field readers.
//!
//! The alliance, participant, body, and overview extractors independently read
//! this object from the original input.

pub(crate) use rokbattles_mail_sdk::{
    ExtractError, require_bool_field, require_child_object, require_number_field, require_object,
    require_string_field, require_u64_field,
};
use serde_json::{Map, Value};

/// Returns the `body.kvs` object from the root payload.
pub(crate) fn require_body_kvs(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "kvs")
}
