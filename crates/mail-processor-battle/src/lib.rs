#![forbid(unsafe_code)]

//! Parses Battle mail reports.

mod content;
mod metadata;
mod opponents;
mod participants;
mod player;
mod sender;
mod summary;
mod timeline;

pub use mail_processor_sdk::{ExtractError, Section};
use mail_processor_sdk::{ProcessError, ProcessedMail, Processor};
use serde_json::Value;

/// Runs the Battle parser with extractors in parallel.
pub fn process_parallel(input: &Value) -> Result<ProcessedMail, ProcessError> {
    processor().process_parallel(input)
}

/// Runs the Battle parser in extractor order.
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
