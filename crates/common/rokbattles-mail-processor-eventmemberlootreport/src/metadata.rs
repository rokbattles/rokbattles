//! Copies common root metadata into the `metadata` object section.
//!
//! Requires string `id` and `receiver` fields and unsigned integer `time` and
//! `serverId` fields. The SDK renames them to `mail_id`, `mail_receiver`,
//! `mail_time`, and `server_id` without converting timestamp units.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

/// Extracts the standard root metadata fields.
#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
    /// Creates a stateless metadata extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        Ok(extract_base_metadata(input)?.into_section())
    }
}
