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
        if let Some(first) = sections.first() {
            if meta.email_id.is_none() {
                meta.email_id = first.get("id").and_then(Self::parse_string);
            }
            if meta.email_type.is_none() {
                meta.email_type = first.get("type").and_then(Self::parse_string);
            }
            if meta.email_box.is_none() {
                meta.email_box = first.get("box").and_then(Self::parse_string);
            }
            if meta.email_time.is_none() {
                meta.email_time = first.get("time").and_then(Self::parse_i64);
            }
            if meta.email_sender.is_none() {
                meta.email_sender = first.get("sender").and_then(Self::parse_string);
            }
            if meta.server_id.is_none() {
                meta.server_id = first.get("serverId").and_then(Self::parse_i64);
            }
        }

        if meta.email_id.is_none() {
            meta.email_id = Self::find_string(sections, "id");
        }
        if meta.email_type.is_none() {
            meta.email_type = Self::find_string(sections, "type");
        }
        if meta.email_box.is_none() {
            meta.email_box = Self::find_string(sections, "box");
        }
        if meta.email_time.is_none() {
            meta.email_time = Self::find_i64(sections, "time");
        }
        if meta.email_sender.is_none() {
            meta.email_sender = Self::find_string(sections, "sender");
        }
        if meta.email_receiver.is_none() {
            meta.email_receiver = Self::find_string(sections, "receiver");
        }
        if meta.server_id.is_none() {
            meta.server_id = Self::find_i64(sections, "serverId");
        }

        Ok(())
    }
}
