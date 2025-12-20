pub mod context;
pub mod error;
pub mod resolvers;
pub mod structures;

use mail_processor_sdk::ResolverChain;
use serde_json::Value;

use crate::context::MailContext;
use crate::error::ProcessError;
use crate::resolvers::MetadataResolver;
use crate::structures::DuelBattle2Mail;

pub fn process_sections(sections: &[Value]) -> Result<DuelBattle2Mail, ProcessError> {
    if sections.is_empty() {
        return Err(ProcessError::EmptySections);
    }

    let ctx = MailContext::new(sections);
    let mut output = DuelBattle2Mail::default();

    let chain = ResolverChain::new().with(MetadataResolver::new());
    chain.apply(&ctx, &mut output)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::process_sections;
    use crate::error::ProcessError;
    use serde_json::json;

    #[test]
    fn process_sections_populates_header_metadata() {
        let sections = vec![json!({
            "id": "mail-1",
            "type": "DuelBattle2",
            "box": "Report",
            "time": 123,
            "sender": "system",
            "serverId": 1804
        })];

        let output = process_sections(&sections).expect("process mail");
        let meta = output.metadata;

        assert_eq!(meta.email_id.as_deref(), Some("mail-1"));
        assert_eq!(meta.email_type.as_deref(), Some("DuelBattle2"));
        assert_eq!(meta.email_box.as_deref(), Some("Report"));
        assert_eq!(meta.email_time, Some(123));
        assert_eq!(meta.email_sender.as_deref(), Some("system"));
        assert_eq!(meta.server_id, Some(1804));
    }

    #[test]
    fn process_sections_scans_for_receiver() {
        let sections = vec![
            json!({
                "id": "mail-2",
                "type": "DuelBattle2",
                "box": "Report",
                "time": 456,
                "sender": "system"
            }),
            json!({
                "receiver": "player_123"
            }),
        ];

        let output = process_sections(&sections).expect("process mail");

        assert_eq!(
            output.metadata.email_receiver.as_deref(),
            Some("player_123")
        );
    }

    #[test]
    fn process_sections_rejects_empty_payloads() {
        let err = process_sections(&[]).unwrap_err();

        assert!(matches!(err, ProcessError::EmptySections));
    }
}
