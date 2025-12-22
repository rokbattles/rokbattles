pub mod context;
pub mod error;
mod helpers;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::{BattleTrendsResolver, DataSummaryResolver, MetadataResolver};
use crate::structures::BattleMail;

/// Processes decoded Battle mail sections into a structured output.
///
/// The input is expected to be the `sections` array from decoded mail JSON.
pub fn process_sections(sections: &[Value]) -> Result<BattleMail, ProcessError> {
    if sections.is_empty() {
        return Err(ProcessError::EmptySections);
    }

    let ctx = MailContext::new(sections);
    let mut output = BattleMail::default();

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(DataSummaryResolver::new())
        .with(BattleTrendsResolver::new());
    chain.apply(&ctx, &mut output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::process_sections;
    use crate::error::ProcessError;

    #[test]
    fn process_sections_rejects_empty_payloads() {
        let err = process_sections(&[]).unwrap_err();

        assert!(matches!(err, ProcessError::EmptySections));
    }
}
