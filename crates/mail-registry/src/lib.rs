//! Shared mail type detection, metadata, and optional processor dispatch.

use std::fmt;

use serde_json::{Map, Value};

/// Internal system barbarian fort mail type label.
pub const MAIL_TYPE_SYSTEM_BARBARIAN_FORT: &str = "SystemBarbarianFort";
/// Internal Ark of Osiris battle results mail type label.
pub const MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS: &str = "AllianceAOOBattleResults";
/// Internal Ark of Osiris battle info mail type label.
pub const MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO: &str = "AllianceAOOBattleInfo";
/// Internal Ark of Osiris individual results mail type label.
pub const MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS: &str = "AllianceAOOIndividualResults";
/// Internal Ark of Osiris registration mail type label.
pub const MAIL_TYPE_ALLIANCE_AOO_REGISTRATION: &str = "AllianceAOORegistration";

/// Supported processable mail categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailType {
    /// Battle reports.
    Battle,
    /// Olympian Arena duel reports.
    DuelBattle2,
    /// Baulur reports.
    BarCanyonKillBoss,
    /// Resource gathering reports.
    Rss,
    /// System barbarian fort reports.
    SystemBarbarianFort,
    /// Ark of Osiris alliance battle results.
    AllianceAOOBattleResults,
    /// Ark of Osiris alliance battle info.
    AllianceAOOBattleInfo,
    /// Ark of Osiris individual results.
    AllianceAOOIndividualResults,
    /// Ark of Osiris registration.
    AllianceAOORegistration,
}

impl MailType {
    /// All processable mail types.
    pub const ALL: [Self; 9] = [
        Self::Battle,
        Self::DuelBattle2,
        Self::BarCanyonKillBoss,
        Self::Rss,
        Self::SystemBarbarianFort,
        Self::AllianceAOOBattleResults,
        Self::AllianceAOOBattleInfo,
        Self::AllianceAOOIndividualResults,
        Self::AllianceAOORegistration,
    ];

    /// Parse a mail type label.
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "Battle" => Some(Self::Battle),
            "DuelBattle2" => Some(Self::DuelBattle2),
            "BarCanyonKillBoss" => Some(Self::BarCanyonKillBoss),
            "Rss" => Some(Self::Rss),
            MAIL_TYPE_SYSTEM_BARBARIAN_FORT => Some(Self::SystemBarbarianFort),
            MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS => Some(Self::AllianceAOOBattleResults),
            MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO => Some(Self::AllianceAOOBattleInfo),
            MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS => Some(Self::AllianceAOOIndividualResults),
            MAIL_TYPE_ALLIANCE_AOO_REGISTRATION => Some(Self::AllianceAOORegistration),
            _ => None,
        }
    }

    /// Parse a mail type label case-insensitively.
    #[must_use]
    pub fn from_label_ignore_ascii_case(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mail_type| mail_type.as_str().eq_ignore_ascii_case(value))
    }

    /// Return the mail type label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Battle => "Battle",
            Self::DuelBattle2 => "DuelBattle2",
            Self::BarCanyonKillBoss => "BarCanyonKillBoss",
            Self::Rss => "Rss",
            Self::SystemBarbarianFort => MAIL_TYPE_SYSTEM_BARBARIAN_FORT,
            Self::AllianceAOOBattleResults => MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS,
            Self::AllianceAOOBattleInfo => MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO,
            Self::AllianceAOOIndividualResults => MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS,
            Self::AllianceAOORegistration => MAIL_TYPE_ALLIANCE_AOO_REGISTRATION,
        }
    }

    /// Return the MongoDB collection name for this mail type.
    #[must_use]
    pub fn collection_name(self) -> &'static str {
        match self {
            Self::Battle => "mails_battle",
            Self::DuelBattle2 => "mails_duelbattle2",
            Self::BarCanyonKillBoss => "mails_barcanyonkillboss",
            Self::Rss => "mails_rss",
            Self::SystemBarbarianFort => "mails_system_barbarianfort",
            Self::AllianceAOOBattleResults => "mails_alliance_aoobattleresults",
            Self::AllianceAOOBattleInfo => "mails_alliance_aoobattleinfo",
            Self::AllianceAOOIndividualResults => "mails_alliance_aooindividualresults",
            Self::AllianceAOORegistration => "mails_alliance_aooregistration",
        }
    }
}

impl fmt::Display for MailType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Normalize a decoded mail payload to a single root object.
#[must_use]
pub fn normalize_mail_root(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(map) => Some(map),
        Value::Array(items) => match items.as_slice() {
            [Value::Object(map)] => Some(map),
            _ => None,
        },
        _ => None,
    }
}

/// Extract the raw mail type string from a decoded mail payload.
#[must_use]
pub fn raw_mail_type(value: &Value) -> Option<&str> {
    let root = normalize_mail_root(value)?;
    root.get("type").and_then(Value::as_str)
}

/// Extract the raw mail type as a displayable string from a decoded mail payload.
#[must_use]
pub fn raw_mail_type_string(value: &Value) -> Option<String> {
    let root = normalize_mail_root(value)?;
    root.get("type").and_then(value_to_string)
}

/// Detect the processable mail type for a normalized root object.
#[must_use]
pub fn detect_mail_type_from_root(root: &Map<String, Value>) -> Option<MailType> {
    let mail_type = root.get("type").and_then(Value::as_str)?;

    if mail_type.eq_ignore_ascii_case("System") && is_system_barbarian_fort_mail(root) {
        return Some(MailType::SystemBarbarianFort);
    }
    if mail_type.eq_ignore_ascii_case("Alliance")
        && let Some(alliance_aoo_type) = detect_alliance_aoo_mail_type(root)
    {
        return Some(alliance_aoo_type);
    }

    MailType::from_label_ignore_ascii_case(mail_type)
}

/// Detect the processable mail type for a decoded mail payload.
#[must_use]
pub fn detect_mail_type(value: &Value) -> Option<MailType> {
    let root = normalize_mail_root(value)?;
    detect_mail_type_from_root(root)
}

/// Return true when the label is one of the processable mail types.
#[must_use]
pub fn is_processable_mail_type(mail_type: &str) -> bool {
    MailType::from_label(mail_type).is_some()
}

/// Return true when the label is supported by the platform.
#[must_use]
pub fn is_supported_mail_type(mail_type: &str) -> bool {
    is_processable_mail_type(mail_type)
}

fn is_system_barbarian_fort_mail(root: &Map<String, Value>) -> bool {
    if !matches!(root.get("box").and_then(Value::as_str), Some("Report")) {
        return false;
    }

    let Some(body) = root.get("body").and_then(Value::as_object) else {
        return false;
    };

    let sub_param = body.get("subParam").and_then(value_as_u64);
    let sub_type = body.get("subType").and_then(value_as_u64);
    matches!(sub_type, Some(11)) && matches!(sub_param, Some(1 | 3))
}

fn detect_alliance_aoo_mail_type(root: &Map<String, Value>) -> Option<MailType> {
    if !matches!(root.get("box").and_then(Value::as_str), Some("AllianceBox")) {
        return None;
    }

    let body = root.get("body").and_then(Value::as_object)?;
    let body_type = body.get("type").and_then(value_as_u64)?;
    let body_param = body.get("param").and_then(value_as_u64);

    match body_type {
        57 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOORegistration),
        14 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOBattleResults),
        15 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOIndividualResults),
        60 => Some(MailType::AllianceAOOBattleResults),
        61 => Some(MailType::AllianceAOOBattleInfo),
        62 => Some(MailType::AllianceAOOIndividualResults),
        _ => None,
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Process a decoded mail root with the registered processor for `mail_type`.
#[cfg(feature = "processors")]
pub fn process_mail(
    mail_type: MailType,
    input: &Value,
) -> Result<mail_processor_sdk::ProcessedMail, mail_processor_sdk::ProcessError> {
    match mail_type {
        MailType::Battle => mail_processor_battle::process(input),
        MailType::DuelBattle2 => mail_processor_duelbattle2::process(input),
        MailType::BarCanyonKillBoss => mail_processor_barcanyonkillboss::process(input),
        MailType::Rss => mail_processor_rss::process(input),
        MailType::SystemBarbarianFort => mail_processor_system_barbarianfort::process(input),
        MailType::AllianceAOOBattleResults => {
            mail_processor_alliance_aoo_battle_results::process(input)
        }
        MailType::AllianceAOOBattleInfo => mail_processor_alliance_aoo_battle_info::process(input),
        MailType::AllianceAOOIndividualResults => {
            mail_processor_alliance_aoo_individual_results::process(input)
        }
        MailType::AllianceAOORegistration => {
            mail_processor_alliance_aoo_registration::process(input)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalize_mail_root_accepts_object_and_singleton_array() {
        let object = json!({ "type": "Battle" });
        assert!(normalize_mail_root(&object).is_some());

        let singleton = json!([{ "type": "Battle" }]);
        assert!(normalize_mail_root(&singleton).is_some());

        let multiple = json!([{ "type": "Battle" }, { "type": "Battle" }]);
        assert!(normalize_mail_root(&multiple).is_none());
    }

    #[test]
    fn detect_mail_type_matches_canonical_types_case_insensitively() {
        assert_eq!(detect_mail_type(&json!({ "type": "Battle" })), Some(MailType::Battle));
        assert_eq!(
            detect_mail_type(&json!({ "type": "duelbattle2" })),
            Some(MailType::DuelBattle2)
        );
        assert_eq!(
            detect_mail_type(&json!({ "type": "BARCANYONKILLBOSS" })),
            Some(MailType::BarCanyonKillBoss)
        );
        assert_eq!(detect_mail_type(&json!({ "type": "rss" })), Some(MailType::Rss));
    }

    #[test]
    fn detect_mail_type_matches_system_barbarian_fort() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 3,
                "subType": 11
            }
        });

        assert_eq!(detect_mail_type(&payload), Some(MailType::SystemBarbarianFort));
    }

    #[test]
    fn detect_mail_type_rejects_unsupported_system_mail() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 2,
                "subType": 11
            }
        });

        assert_eq!(detect_mail_type(&payload), None);
    }

    #[test]
    fn detect_mail_type_matches_alliance_aoo_variants() {
        let registration = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 57, "param": 1 }
        });
        assert_eq!(detect_mail_type(&registration), Some(MailType::AllianceAOORegistration));

        let custom_battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 1 }
        });
        assert_eq!(
            detect_mail_type(&custom_battle_results),
            Some(MailType::AllianceAOOBattleResults)
        );

        let battle_info = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 61 }
        });
        assert_eq!(detect_mail_type(&battle_info), Some(MailType::AllianceAOOBattleInfo));

        let individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 62 }
        });
        assert_eq!(
            detect_mail_type(&individual_results),
            Some(MailType::AllianceAOOIndividualResults)
        );
    }

    #[test]
    fn detect_mail_type_rejects_aoo_custom_types_with_other_params() {
        let battle_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 14, "param": 2 }
        });
        assert_eq!(detect_mail_type(&battle_results), None);

        let individual_results = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 15, "param": 3 }
        });
        assert_eq!(detect_mail_type(&individual_results), None);
    }

    #[test]
    fn detect_mail_type_rejects_partial_alliance_matches() {
        let missing_box = json!({
            "type": "Alliance",
            "body": { "type": 60, "param": 1 }
        });
        assert_eq!(detect_mail_type(&missing_box), None);

        let missing_body = json!({
            "type": "Alliance",
            "box": "AllianceBox"
        });
        assert_eq!(detect_mail_type(&missing_body), None);
    }

    #[test]
    fn detect_mail_type_rejects_non_aoo_alliance_mail() {
        let payload = json!({
            "type": "Alliance",
            "box": "AllianceBox",
            "body": { "type": 99 }
        });

        assert_eq!(detect_mail_type(&payload), None);
    }

    #[test]
    fn detect_mail_type_rejects_partial_system_matches() {
        let unsupported_box = json!({
            "type": "System",
            "box": "AllianceBox",
            "body": { "subType": 11, "subParam": 1 }
        });
        assert_eq!(detect_mail_type(&unsupported_box), None);

        let unsupported_sub_type = json!({
            "type": "System",
            "box": "Report",
            "body": { "subType": 10, "subParam": 1 }
        });
        assert_eq!(detect_mail_type(&unsupported_sub_type), None);
    }

    #[test]
    fn raw_mail_type_reads_only_string_type() {
        assert_eq!(raw_mail_type(&json!({ "type": "DuelBattle2" })), Some("DuelBattle2"));
        assert_eq!(raw_mail_type(&json!({ "type": 12 })), None);
    }

    #[test]
    fn raw_mail_type_string_reads_numeric_type() {
        assert_eq!(raw_mail_type_string(&json!({ "type": 12 })).as_deref(), Some("12"));
    }

    #[test]
    fn collection_name_matches_expected() {
        assert_eq!(MailType::Battle.collection_name(), "mails_battle");
        assert_eq!(MailType::DuelBattle2.collection_name(), "mails_duelbattle2");
        assert_eq!(MailType::BarCanyonKillBoss.collection_name(), "mails_barcanyonkillboss");
        assert_eq!(MailType::Rss.collection_name(), "mails_rss");
        assert_eq!(MailType::SystemBarbarianFort.collection_name(), "mails_system_barbarianfort");
        assert_eq!(
            MailType::AllianceAOOBattleResults.collection_name(),
            "mails_alliance_aoobattleresults"
        );
        assert_eq!(
            MailType::AllianceAOOBattleInfo.collection_name(),
            "mails_alliance_aoobattleinfo"
        );
        assert_eq!(
            MailType::AllianceAOOIndividualResults.collection_name(),
            "mails_alliance_aooindividualresults"
        );
        assert_eq!(
            MailType::AllianceAOORegistration.collection_name(),
            "mails_alliance_aooregistration"
        );
    }
}
