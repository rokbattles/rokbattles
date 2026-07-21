//! Helpers for walking BarCanyonKillBoss mail content.

pub(crate) use rokbattles_mail_sdk::{
    ExtractError, require_child_object, require_number_field, require_object, require_string_field,
    require_u64_field,
};
use serde_json::{Map, Value};

/// Returns the nested BarCanyonKillBoss content object.
pub(crate) fn require_content(input: &Value) -> Result<&Map<String, Value>, ExtractError> {
    let root = require_object(input)?;
    let body = require_child_object(root, "body")?;
    require_child_object(body, "content")
}
