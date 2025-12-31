use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::Value;

use crate::context::MailContext;
use crate::structures::DuelBattle2Mail;

/// Resolves the mail header metadata for DuelBattle2 reports.
pub struct MetadataResolver;

impl Default for MetadataResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataResolver {
    pub fn new() -> Self {
        Self
    }

    fn parse_i128(v: &Value) -> Option<i128> {
        match v {
            Value::Number(n) => n
                .as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from)),
            Value::String(s) => s.trim().parse::<i128>().ok(),
            _ => None,
        }
    }

    fn parse_i64(v: &Value) -> Option<i64> {
        Self::parse_i128(v).and_then(|n| i64::try_from(n).ok())
    }

    fn parse_string(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.to_owned()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }

    fn find_string(sections: &[Value], key: &str) -> Option<String> {
        sections
            .iter()
            .find_map(|section| section.get(key).and_then(Self::parse_string))
    }

    fn find_i64(sections: &[Value], key: &str) -> Option<i64> {
        sections
            .iter()
            .find_map(|section| section.get(key).and_then(Self::parse_i64))
    }
}

impl Resolver<MailContext<'_>, DuelBattle2Mail> for MetadataResolver {
    type Error = Infallible;

    fn resolve(
        &self,
        ctx: &MailContext<'_>,
        output: &mut DuelBattle2Mail,
    ) -> Result<(), Self::Error> {
        let sections = ctx.sections;
        let meta = &mut output.metadata;

        // Prefer the first section because it usually carries the mail header.
        let first = sections.first();
        meta.email_id = first
            .and_then(|section| section.get("id").and_then(Self::parse_string))
            .or_else(|| Self::find_string(sections, "id"))
            .unwrap_or_default();
        meta.email_time = first
            .and_then(|section| section.get("time").and_then(Self::parse_i64))
            .or_else(|| Self::find_i64(sections, "time"))
            .unwrap_or_default();
        meta.email_receiver = Self::find_string(sections, "receiver").unwrap_or_default();
        meta.server_id = first
            .and_then(|section| section.get("serverId").and_then(Self::parse_i64))
            .or_else(|| Self::find_i64(sections, "serverId"))
            .unwrap_or_default();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MetadataResolver;
    use crate::context::MailContext;
    use crate::structures::DuelBattle2Mail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_metadata(sections: Vec<Value>) -> DuelBattle2Mail {
        let ctx = MailContext::new(&sections);
        let mut output = DuelBattle2Mail::default();
        let resolver = MetadataResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve metadata");

        output
    }

    #[test]
    fn metadata_resolver_populates_header_metadata() {
        let sections = vec![json!({
            "id": "mail-1",
            "time": 123,
            "serverId": 1804
        })];

        let output = resolve_metadata(sections);
        let meta = output.metadata;

        assert_eq!(meta.email_id, "mail-1");
        assert_eq!(meta.email_time, 123);
        assert_eq!(meta.server_id, 1804);
    }

    #[test]
    fn metadata_resolver_scans_for_receiver() {
        let sections = vec![
            json!({
                "id": "mail-2",
                "time": 456,
            }),
            json!({
                "receiver": "player_123"
            }),
        ];

        let output = resolve_metadata(sections);

        assert_eq!(output.metadata.email_receiver, "player_123");
    }
}
