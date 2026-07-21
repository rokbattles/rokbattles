#![forbid(unsafe_code)]

//! Parses RSS mail reports.

mod content;
mod metadata;
mod rss;

pub use rokbattles_codegen_mail_types::rss::Rss;
pub use rokbattles_mail_sdk::{ExtractError, Section};
use rokbattles_mail_sdk::{ProcessError, Processor};
use serde_json::Value;

/// Runs the RSS parser.
pub fn process(input: &Value) -> Result<Rss, ProcessError> {
    processor().process(input)?.into_typed()
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(rss::RssExtractor::new()),
    ])
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    #[test]
    fn process_extracts_expected_sections() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Rss/Persistent.Mail.113157979177212756131.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert_eq!(processed.metadata.mail_id, "113157979177212756131");
    }

    #[test]
    fn process_extracts_sample_without_crystals_gain() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../samples/Rss/Persistent.Mail.118801516499340535.json");
        let json = fs::read_to_string(sample_path).expect("read sample");
        let value: Value = serde_json::from_str(&json).expect("parse sample");

        let processed = process(&value).expect("process sample");

        assert_eq!(processed.rss.crystals_gain.as_u64(), Some(0));
    }
}
