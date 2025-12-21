use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::Value;

use crate::context::MailContext;
use crate::structures::BattleMail;

/// Resolves the mail header metadata for Battle reports.
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

    fn parse_bool(v: &Value) -> Option<bool> {
        match v {
            Value::Bool(b) => Some(*b),
            Value::Number(n) => n.as_i64().map(|x| x != 0),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            },
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

    fn find_string_with_body(sections: &[Value], key: &str) -> Option<String> {
        sections.iter().find_map(|section| {
            section.get(key).and_then(Self::parse_string).or_else(|| {
                section
                    .get("body")
                    .and_then(|body| body.get(key))
                    .and_then(Self::parse_string)
            })
        })
    }

    fn find_bool_with_body(sections: &[Value], key: &str) -> Option<bool> {
        sections.iter().find_map(|section| {
            section.get(key).and_then(Self::parse_bool).or_else(|| {
                section
                    .get("body")
                    .and_then(|body| body.get(key))
                    .and_then(Self::parse_bool)
            })
        })
    }

    fn find_sender_cos_id(sections: &[Value]) -> Option<i64> {
        sections.iter().find_map(|section| {
            section
                .get("COSId")
                .and_then(Self::parse_i64)
                .or_else(|| {
                    section
                        .get("body")
                        .and_then(|body| body.get("COSId"))
                        .and_then(Self::parse_i64)
                })
                .or_else(|| {
                    section
                        .pointer("/body/content/SelfChar/COSId")
                        .and_then(Self::parse_i64)
                })
                .or_else(|| {
                    section
                        .pointer("/body/content/COSId")
                        .and_then(Self::parse_i64)
                })
                .or_else(|| {
                    section
                        .pointer("/content/SelfChar/COSId")
                        .and_then(Self::parse_i64)
                })
        })
    }

    /// Classify the battle report type for ROK Battles metadata output.
    fn compute_rokb_email_type(
        role: Option<String>,
        is_conquer_season: Option<bool>,
        server_id: Option<i64>,
        sender_cos_id: Option<i64>,
    ) -> Option<String> {
        let role = role?.trim().to_ascii_lowercase();

        // Role = dungeon always maps to ark, regardless of other flags.
        if role == "dungeon" {
            return Some("ark".to_string());
        }

        let is_gs_role = matches!(role.as_str(), "gsmp" | "gs");
        if !is_gs_role {
            return None;
        }

        if is_conquer_season == Some(true) {
            return Some("kvk".to_string());
        }

        // Conquer season flags are not always present, so fall back to server vs sender kingdom IDs.
        match (server_id, sender_cos_id) {
            (Some(server_id), Some(cos_id)) if server_id != 0 && cos_id != 0 => {
                if server_id == cos_id {
                    Some("home".to_string())
                } else {
                    Some("kvk".to_string())
                }
            }
            _ => None,
        }
    }

    /// Classify battle type based on sender participant sections.
    fn compute_rokb_battle_type(sections: &[Value]) -> Option<String> {
        let mut sender_seen = false;
        let mut sender_has_abt = false;
        let mut sender_has_rally = false;

        for section in sections {
            let pname = section.get("PName").and_then(Self::parse_string);
            if pname
                .as_deref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                continue;
            }

            let is_rally_flag = section.get("IsRally").and_then(Self::parse_bool);
            let is_rally_participant = is_rally_flag.unwrap_or(false);

            // Sender sections appear first; once we hit a non-rally participant after
            // the sender, the rest are opponent data.
            if sender_seen && !is_rally_participant {
                break;
            }

            sender_seen = true;

            if !sender_has_abt && section.get("AbT").and_then(Self::parse_i64).is_some() {
                sender_has_abt = true;
            }

            if !sender_has_rally && is_rally_flag == Some(true) {
                sender_has_rally = true;
            }
        }

        if !sender_seen {
            // Some reports only embed the sender in the mail header body content.
            let content = sections
                .first()
                .and_then(|section| section.get("body"))
                .and_then(|body| body.get("content"))?;
            let pname = content.get("PName").and_then(Self::parse_string);
            if pname
                .as_deref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                return None;
            }

            let has_abt = content.get("AbT").and_then(Self::parse_i64).is_some();
            let has_rally = content.get("IsRally").and_then(Self::parse_bool) == Some(true);

            return Some(
                match (has_abt, has_rally) {
                    (true, _) => "garrison",
                    (false, true) => "rally",
                    (false, false) => "open_field",
                }
                .to_string(),
            );
        }

        if sender_has_abt {
            return Some("garrison".to_string());
        }

        if sender_has_rally {
            return Some("rally".to_string());
        }

        Some("open_field".to_string())
    }
}

impl Resolver<MailContext<'_>, BattleMail> for MetadataResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let sections = ctx.sections;
        let meta = &mut output.metadata;

        // Prefer the first section because it usually carries the mail header.
        if let Some(first) = sections.first() {
            if meta.email_id.is_none() {
                meta.email_id = first.get("id").and_then(Self::parse_string);
            }
            if meta.email_time.is_none() {
                meta.email_time = first.get("time").and_then(Self::parse_i64);
            }
            if meta.server_id.is_none() {
                meta.server_id = first.get("serverId").and_then(Self::parse_i64);
            }
        }

        if meta.email_id.is_none() {
            meta.email_id = Self::find_string(sections, "id");
        }
        if meta.email_time.is_none() {
            meta.email_time = Self::find_i64(sections, "time");
        }
        if meta.email_receiver.is_none() {
            meta.email_receiver = Self::find_string(sections, "receiver");
        }
        if meta.server_id.is_none() {
            meta.server_id = Self::find_i64(sections, "serverId");
        }

        if meta.rokb_email_type.is_none() {
            let role = Self::find_string_with_body(sections, "Role");
            let is_conquer_season = Self::find_bool_with_body(sections, "isConquerSeason");
            let sender_cos_id = Self::find_sender_cos_id(sections);
            let server_id = meta
                .server_id
                .or_else(|| Self::find_i64(sections, "serverId"));

            meta.rokb_email_type =
                Self::compute_rokb_email_type(role, is_conquer_season, server_id, sender_cos_id);
        }

        if meta.rokb_battle_type.is_none() {
            meta.rokb_battle_type = Self::compute_rokb_battle_type(sections);
        }

        Ok(())
    }
}
