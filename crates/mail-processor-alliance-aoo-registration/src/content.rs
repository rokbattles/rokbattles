//! Helpers for walking AllianceAOORegistration mail content.

use mail_sdk::{ExtractError, require_child_object, require_object};
use serde_json::{Map, Value};

/// Returns the registration payload nested under `body.kvs`.
pub fn require_body_kvs(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "kvs")
}
