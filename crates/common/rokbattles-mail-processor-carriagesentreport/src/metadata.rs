//! Extracts the mail metadata shared by carriage and other reports.

use rokbattles_mail_sdk::{ExtractError, Extractor, Section, extract_base_metadata};
use serde_json::Value;

pub(super) struct MetadataExtractor;

impl Extractor for MetadataExtractor {
    fn section(&self) -> &'static str {
        "metadata"
    }

    fn extract(&self, input: &Value) -> Result<Section, ExtractError> {
        Ok(extract_base_metadata(input)?.into_section())
    }
}
