#![forbid(unsafe_code)]

//! Processor for Battle mail reports.

mod content;
mod metadata;
mod opponents;
mod participants;
mod player;
mod sender;
mod summary;
mod timeline;

use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

pub use mail_processor_sdk::{ExtractError, Section};

/// Process a decoded Battle mail with parallel extractors.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Process a decoded Battle mail in extractor order.
pub fn process_sequential(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_sequential(input)
}

fn processor() -> Processor {
    Processor::new(vec![
        Box::new(metadata::MetadataExtractor::new()),
        Box::new(sender::SenderExtractor::new()),
        Box::new(summary::SummaryExtractor::new()),
        Box::new(opponents::OpponentsExtractor::new()),
        Box::new(timeline::TimelineExtractor::new()),
    ])
}
