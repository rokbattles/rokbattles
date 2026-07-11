//! Metadata parser for GVE member loot reports.

use mail_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

#[derive(Debug, Default)]
pub struct MetadataExtractor;

impl MetadataExtractor {
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
