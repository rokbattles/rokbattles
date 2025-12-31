pub mod context;
pub mod error;
mod helpers;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::{DetailsResolver, MetadataResolver};
use crate::structures::DuelBattle2Mail;

/// Processes decoded DuelBattle2 mail sections into a structured output.
///
/// The input is expected to be the `sections` array from decoded mail JSON.
pub fn process_sections(sections: &[Value]) -> Result<DuelBattle2Mail, ProcessError> {
    if sections.is_empty() {
        return Err(ProcessError::EmptySections);
    }

    let ctx = MailContext::new(sections);
    let mut output = DuelBattle2Mail::default();

    let chain = ResolverChain::new()
        .with(MetadataResolver::new())
        .with(DetailsResolver::new());
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
