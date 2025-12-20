use mail_decoder::Mail;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum EmailType {
    Alliance,
    AllianceApply,
    AllianceBuilding,
    ArenaRankRewardReport,
    Battle,
    CarriageSentReport,
    DuelBattle2,
    Event,
    EventAsRank,
    EventMemberLootReport,
    EventPlyRank,
    Gm,
    KillBigDreamReport,
    Mlang,
    Player,
    Rss,
    ScoutReport,
    System,
    TeamGachaResult,
    Temple,
    Unknown(String),
}

impl EmailType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "Alliance" => Self::Alliance,
            "AllianceApply" => Self::AllianceApply,
            "AllianceBuilding" => Self::AllianceBuilding,
            "ArenaRankRewardReport" => Self::ArenaRankRewardReport,
            "Battle" => Self::Battle,
            "CarriageSentReport" => Self::CarriageSentReport,
            "DuelBattle2" => Self::DuelBattle2,
            "Event" => Self::Event,
            "EventAsRank" => Self::EventAsRank,
            "EventMemberLootReport" => Self::EventMemberLootReport,
            "EventPlyRank" => Self::EventPlyRank,
            "Gm" => Self::Gm,
            "KillBigDreamReport" => Self::KillBigDreamReport,
            "Mlang" => Self::Mlang,
            "Player" => Self::Player,
            "Rss" => Self::Rss,
            "ScoutReport" => Self::ScoutReport,
            "System" => Self::System,
            "TeamGachaResult" => Self::TeamGachaResult,
            "Temple" => Self::Temple,
            other => Self::Unknown(other.to_string()),
        }
    }
}

pub fn detect_mail_type_str(mail: &Mail) -> Option<&str> {
    for section in &mail.sections {
        if let Some(obj) = section.as_object()
            && let Some(Value::String(t)) = obj.get("type")
        {
            // Use the first section that declares a string "type".
            return Some(t.as_str());
        }
    }
    None
}

pub fn detect_mail_type(mail: &Mail) -> Option<EmailType> {
    detect_mail_type_str(mail).map(EmailType::from_str)
}

pub fn detect_mail_time(mail: &Mail) -> Option<i64> {
    for section in &mail.sections {
        if let Some(obj) = section.as_object()
            && let Some(Value::Number(n)) = obj.get("time")
            && let Some(i) = n.as_i64()
        {
            // Return the first numeric epoch timestamp we encounter.
            return Some(i);
        }
    }
    None
}

pub fn detect_email_id(mail: &Mail) -> Option<String> {
    for section in &mail.sections {
        if let Some(obj) = section.as_object() {
            if let Some(Value::String(s)) = obj.get("id") {
                // Prefer string ids if present.
                return Some(s.clone());
            }
            // TODO need to verify this is the case ever
            if let Some(Value::Number(n)) = obj.get("id") {
                // Fallback: stringify numeric ids.
                return Some(n.to_string());
            }
        }
    }
    None
}
