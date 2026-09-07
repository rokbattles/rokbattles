//! Identifies supported mail categories and maps them to labels and collections.
//!
//! Use [`detect_mail_type`] to classify a decoded JSON payload, then use
//! [`MailType`] for its canonical label or MongoDB collection name. Detection
//! inspects an object root and selected discriminator fields; it does not
//! validate the complete payload expected by a processor.
//!
//! # Detection
//!
//! Most categories match the root `type` string, ignoring ASCII case. System
//! and Alliance mail use broader game labels, so their category also depends
//! on fields in `body`:
//!
//! | Root `type` | Body fields | Category |
//! | --- | --- | --- |
//! | `System` | `subType = 11`, `subParam = 1`, `3`, or `4` | [`MailType::SystemBarbarianFort`] |
//! | `System` | `subType = 29`, `subParam = 11` | [`MailType::SystemKaharTreasure`] |
//! | `Alliance` | `type = 14`, `param = 1`, or `type = 60` | [`MailType::AllianceAOOBattleResults`] |
//! | `Alliance` | `type = 61` | [`MailType::AllianceAOOBattleInfo`] |
//! | `Alliance` | `type = 15`, `param = 1`, or `type = 62` | [`MailType::AllianceAOOIndividualResults`] |
//!
//! These numeric fields accept JSON unsigned integers or strings parsed as
//! `u64`. Alliance body types 60–62 do not require `param`. Mailbox fields such
//! as `box` and `prevBox` do not affect detection, so archived mail follows the
//! same rules.
//!
//! `EventMemberLootReport` also requires `body.content.EventName` to be `GVE`,
//! ignoring ASCII case. Other event names are unsupported. The canonical
//! System and Alliance labels returned by [`MailType::as_str`] can also appear
//! directly in the root `type`; those labels do not require subtype checks.
//!
//! # Examples
//!
//! Detection maps the game's broad label to a category used by the application:
//!
//! ```
//! use rokbattles_mail_registry::{MailType, detect_mail_type, raw_mail_type};
//! use serde_json::json;
//!
//! let mail = json!({
//!     "type": "System",
//!     "body": { "subType": 29, "subParam": 11 }
//! });
//! assert_eq!(raw_mail_type(&mail), Some("System"));
//! let category = detect_mail_type(&mail).expect("recognized subtype");
//! assert_eq!(category, MailType::SystemKaharTreasure);
//! assert_eq!(category.as_str(), "SystemKaharTreasure");
//! assert_eq!(category.collection_name(), "mails_system_kahartreasure");
//! ```
//!
//! # Features
//!
//! The default build provides detection and metadata without processor
//! dependencies. Enable `processors` to expose `process_mail`, which forwards
//! the payload to the processor selected by the caller's [`MailType`]. The
//! registered categories are the same with or without this feature.

use std::fmt;

use serde_json::{Map, Value};

/// Canonical registry label for [`MailType::SystemBarbarianFort`].
pub const MAIL_TYPE_SYSTEM_BARBARIAN_FORT: &str = "SystemBarbarianFort";
/// Canonical registry label for [`MailType::SystemKaharTreasure`].
pub const MAIL_TYPE_SYSTEM_KAHAR_TREASURE: &str = "SystemKaharTreasure";
/// Canonical registry label for [`MailType::AllianceAOOBattleResults`].
pub const MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS: &str = "AllianceAOOBattleResults";
/// Canonical registry label for [`MailType::AllianceAOOBattleInfo`].
pub const MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO: &str = "AllianceAOOBattleInfo";
/// Canonical registry label for [`MailType::AllianceAOOIndividualResults`].
pub const MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS: &str = "AllianceAOOIndividualResults";
/// A registered mail category with a canonical label and collection name.
///
/// Labels identify application categories and may differ from the game's root
/// `type` string. Parse a known category label with [`Self::from_label`], or
/// inspect a payload with [`detect_mail_type`]. [`Display`](fmt::Display)
/// writes the same canonical label as [`Self::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailType {
    /// Battle reports.
    Battle,
    /// Olympian Arena duel reports.
    DuelBattle2,
    /// Baulur reports.
    BarCanyonKillBoss,
    /// GVE alliance boss member loot reports.
    EventMemberLootReport,
    /// Resource gathering reports.
    Rss,
    /// System barbarian fort reports.
    SystemBarbarianFort,
    /// System Kahar treasure reward mail.
    SystemKaharTreasure,
    /// Ark of Osiris alliance battle results.
    AllianceAOOBattleResults,
    /// Ark of Osiris alliance battle info.
    AllianceAOOBattleInfo,
    /// Ark of Osiris individual results.
    AllianceAOOIndividualResults,
}

impl MailType {
    /// All categories registered for processing, in registry order.
    ///
    /// Available even when the `processors` feature is disabled.
    pub const ALL: [Self; 10] = [
        Self::Battle,
        Self::DuelBattle2,
        Self::BarCanyonKillBoss,
        Self::EventMemberLootReport,
        Self::Rss,
        Self::SystemBarbarianFort,
        Self::SystemKaharTreasure,
        Self::AllianceAOOBattleResults,
        Self::AllianceAOOBattleInfo,
        Self::AllianceAOOIndividualResults,
    ];

    /// Parses an exact, case-sensitive canonical label.
    ///
    /// Returns `None` for unknown labels, including the broad game labels
    /// `System` and `Alliance`. Does not trim whitespace or inspect a payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use rokbattles_mail_registry::MailType;
    ///
    /// assert_eq!(MailType::from_label("Battle"), Some(MailType::Battle));
    /// assert_eq!(MailType::from_label("battle"), None);
    /// assert_eq!(MailType::from_label("System"), None);
    /// ```
    #[must_use]
    pub fn from_label(value: &str) -> Option<Self> {
        match value {
            "Battle" => Some(Self::Battle),
            "DuelBattle2" => Some(Self::DuelBattle2),
            "BarCanyonKillBoss" => Some(Self::BarCanyonKillBoss),
            "EventMemberLootReport" => Some(Self::EventMemberLootReport),
            "Rss" => Some(Self::Rss),
            MAIL_TYPE_SYSTEM_BARBARIAN_FORT => Some(Self::SystemBarbarianFort),
            MAIL_TYPE_SYSTEM_KAHAR_TREASURE => Some(Self::SystemKaharTreasure),
            MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS => Some(Self::AllianceAOOBattleResults),
            MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO => Some(Self::AllianceAOOBattleInfo),
            MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS => Some(Self::AllianceAOOIndividualResults),
            _ => None,
        }
    }

    /// Parses a canonical label, ignoring ASCII case.
    ///
    /// Uses the same labels as [`Self::from_label`] and returns `None` for an
    /// unknown label. Whitespace is not trimmed.
    #[must_use]
    pub fn from_label_ignore_ascii_case(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mail_type| mail_type.as_str().eq_ignore_ascii_case(value))
    }

    /// Returns the canonical label accepted by [`Self::from_label`].
    ///
    /// System and Alliance subcategories use registry labels rather than the
    /// broad `System` or `Alliance` labels present in game mail.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Battle => "Battle",
            Self::DuelBattle2 => "DuelBattle2",
            Self::BarCanyonKillBoss => "BarCanyonKillBoss",
            Self::EventMemberLootReport => "EventMemberLootReport",
            Self::Rss => "Rss",
            Self::SystemBarbarianFort => MAIL_TYPE_SYSTEM_BARBARIAN_FORT,
            Self::SystemKaharTreasure => MAIL_TYPE_SYSTEM_KAHAR_TREASURE,
            Self::AllianceAOOBattleResults => MAIL_TYPE_ALLIANCE_AOO_BATTLE_RESULTS,
            Self::AllianceAOOBattleInfo => MAIL_TYPE_ALLIANCE_AOO_BATTLE_INFO,
            Self::AllianceAOOIndividualResults => MAIL_TYPE_ALLIANCE_AOO_INDIVIDUAL_RESULTS,
        }
    }

    /// Returns the MongoDB collection name assigned to this category.
    ///
    /// This is metadata only; the registry does not access the database.
    #[must_use]
    pub fn collection_name(self) -> &'static str {
        match self {
            Self::Battle => "mails_battle",
            Self::DuelBattle2 => "mails_duelbattle2",
            Self::BarCanyonKillBoss => "mails_barcanyonkillboss",
            Self::EventMemberLootReport => "mails_eventmemberlootreport",
            Self::Rss => "mails_rss",
            Self::SystemBarbarianFort => "mails_system_barbarianfort",
            Self::SystemKaharTreasure => "mails_system_kahartreasure",
            Self::AllianceAOOBattleResults => "mails_alliance_aoobattleresults",
            Self::AllianceAOOBattleInfo => "mails_alliance_aoobattleinfo",
            Self::AllianceAOOIndividualResults => "mails_alliance_aooindividualresults",
        }
    }
}

impl fmt::Display for MailType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Returns `value` unchanged if its root is a JSON object.
///
/// Returns `None` for every other shape, including an array containing a single
/// object. This checks only the root shape; it neither unwraps containers nor
/// validates fields inside the object. The result borrows from `value`.
#[must_use]
pub fn normalize_mail_root(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(_) => Some(value),
        _ => None,
    }
}

/// Borrows the root `type` string from a decoded JSON object.
///
/// Returns `None` if the root is not an object or `type` is missing or not a
/// string. The text is returned unchanged, including unknown labels and their
/// original casing. Use [`detect_mail_type`] to resolve a supported category.
#[must_use]
pub fn raw_mail_type(value: &Value) -> Option<&str> {
    let root = normalize_mail_root(value)?;
    root.get("type").and_then(Value::as_str)
}

/// Copies the root `type` string or renders a numeric `type` as JSON number text.
///
/// Returns `None` for a non-object root, a missing field, or any field type
/// other than string or number. Useful for displaying unrecognized mail types;
/// accepting a number here does not make it valid for [`detect_mail_type`].
#[must_use]
pub fn raw_mail_type_string(value: &Value) -> Option<String> {
    let root = normalize_mail_root(value)?;
    root.get("type").and_then(value_to_string)
}

/// Detects a registered category from a decoded root object.
///
/// Requires a string `type` and applies the [detection rules](crate#detection).
/// Returns `None` for unknown labels or unmatched System, Alliance, or GVE
/// discriminator fields. Root field names are case-sensitive; recognized label
/// values and the GVE event name are compared ignoring ASCII case.
///
/// A successful match identifies a processor category, not a fully valid mail.
/// Canonical System and Alliance category labels are accepted directly without
/// inspecting their bodies.
#[must_use]
pub fn detect_mail_type_from_root(root: &Map<String, Value>) -> Option<MailType> {
    let mail_type = root.get("type").and_then(Value::as_str)?;

    // Resolve the game's broad labels before trying the registry's canonical labels.
    if mail_type.eq_ignore_ascii_case("System") {
        if is_system_kahar_treasure_mail(root) {
            return Some(MailType::SystemKaharTreasure);
        }
        if is_system_barbarian_fort_mail(root) {
            return Some(MailType::SystemBarbarianFort);
        }
    }
    if mail_type.eq_ignore_ascii_case("Alliance")
        && let Some(alliance_aoo_type) = detect_alliance_aoo_mail_type(root)
    {
        return Some(alliance_aoo_type);
    }
    // Return the content check directly so non-GVE reports cannot pass through
    // the canonical-label fallback below.
    if mail_type.eq_ignore_ascii_case("EventMemberLootReport") {
        return is_gve_event_member_loot_report(root).then_some(MailType::EventMemberLootReport);
    }

    MailType::from_label_ignore_ascii_case(mail_type)
}

/// Detects a registered category from a decoded JSON payload.
///
/// Returns `None` if the root is not an object or its category is not recognized.
/// Delegates to [`detect_mail_type_from_root`] after checking the root shape.
/// See the [crate example](crate#examples) for subtype detection.
#[must_use]
pub fn detect_mail_type(value: &Value) -> Option<MailType> {
    let root = normalize_mail_root(value)?.as_object()?;
    detect_mail_type_from_root(root)
}

/// Returns whether `mail_type` is an exact canonical registry label.
///
/// Equivalent to [`MailType::from_label`] returning `Some`. This check is
/// case-sensitive and does not validate a payload: `EventMemberLootReport`
/// is a recognized label even when a particular report is not a GVE report.
#[must_use]
pub fn is_processable_mail_type(mail_type: &str) -> bool {
    MailType::from_label(mail_type).is_some()
}

/// Returns the same result as [`is_processable_mail_type`].
///
/// Support here means case-sensitive registry membership, independent of
/// whether this build enables processor dispatch.
#[must_use]
pub fn is_supported_mail_type(mail_type: &str) -> bool {
    is_processable_mail_type(mail_type)
}

fn is_system_barbarian_fort_mail(root: &Map<String, Value>) -> bool {
    let Some(body) = root.get("body").and_then(Value::as_object) else {
        return false;
    };

    let sub_param = body.get("subParam").and_then(value_as_u64);
    let sub_type = body.get("subType").and_then(value_as_u64);

    matches!((sub_type, sub_param), (Some(11), Some(1 | 3 | 4)))
}

fn is_gve_event_member_loot_report(root: &Map<String, Value>) -> bool {
    root.get("body")
        .and_then(Value::as_object)
        .and_then(|body| body.get("content"))
        .and_then(Value::as_object)
        .and_then(|content| content.get("EventName"))
        .and_then(Value::as_str)
        .is_some_and(|event_name| event_name.eq_ignore_ascii_case("GVE"))
}

fn is_system_kahar_treasure_mail(root: &Map<String, Value>) -> bool {
    let Some(body) = root.get("body").and_then(Value::as_object) else {
        return false;
    };

    let sub_param = body.get("subParam").and_then(value_as_u64);
    let sub_type = body.get("subType").and_then(value_as_u64);

    matches!((sub_type, sub_param), (Some(29), Some(11)))
}

fn detect_alliance_aoo_mail_type(root: &Map<String, Value>) -> Option<MailType> {
    let body = root.get("body").and_then(Value::as_object)?;
    let body_type = body.get("type").and_then(value_as_u64)?;
    let body_param = body.get("param").and_then(value_as_u64);

    // Types 14 and 15 need param 1 to identify AoO mail; types 60–62 are
    // sufficient on their own.
    match body_type {
        14 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOBattleResults),
        15 if matches!(body_param, Some(1)) => Some(MailType::AllianceAOOIndividualResults),
        60 => Some(MailType::AllianceAOOBattleResults),
        61 => Some(MailType::AllianceAOOBattleInfo),
        62 => Some(MailType::AllianceAOOIndividualResults),
        _ => None,
    }
}

// Discriminators may arrive as integers or decimal strings. Floating-point
// JSON numbers are not coerced to integers here.
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

/// Processes `input` with the processor registered for `mail_type`.
///
/// Available with the `processors` feature. The category is supplied by the
/// caller; this function does not detect it or normalize `input`. Pass the
/// decoded object root, typically after calling [`detect_mail_type`].
///
/// Returns the selected processor's structured mail output. The processor
/// interprets and validates the fields it needs.
///
/// # Errors
///
/// Returns the selected processor's [`rokbattles_mail_sdk::ProcessError`]
/// unchanged if processing fails. The exact required fields and validation
/// rules depend on the selected category.
#[cfg(feature = "processors")]
pub fn process_mail(
    mail_type: MailType,
    input: &Value,
) -> Result<rokbattles_mail_sdk::ProcessedMail, rokbattles_mail_sdk::ProcessError> {
    match mail_type {
        MailType::Battle => rokbattles_mail_processor_battle::process(input),
        MailType::DuelBattle2 => rokbattles_mail_processor_duelbattle2::process(input),
        MailType::BarCanyonKillBoss => rokbattles_mail_processor_barcanyonkillboss::process(input),
        MailType::EventMemberLootReport => {
            rokbattles_mail_processor_eventmemberlootreport::process(input)
        }
        MailType::Rss => rokbattles_mail_processor_rss::process(input),
        MailType::SystemBarbarianFort => {
            rokbattles_mail_processor_system_barbarianfort::process(input)
        }
        MailType::SystemKaharTreasure => {
            rokbattles_mail_processor_system_kahartreasure::process(input)
        }
        MailType::AllianceAOOBattleResults => {
            rokbattles_mail_processor_alliance_aoo_battle_results::process(input)
        }
        MailType::AllianceAOOBattleInfo => {
            rokbattles_mail_processor_alliance_aoo_battle_info::process(input)
        }
        MailType::AllianceAOOIndividualResults => {
            rokbattles_mail_processor_alliance_aoo_individual_results::process(input)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalize_mail_root_accepts_object() {
        let object = json!({ "type": "Battle" });
        assert_eq!(normalize_mail_root(&object), Some(&object));
    }

    #[test]
    fn normalize_mail_root_rejects_singleton_array() {
        let singleton = json!([{ "type": "Battle" }]);
        assert_eq!(normalize_mail_root(&singleton), None);
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
    fn detect_mail_type_accepts_only_gve_member_loot_reports() {
        let gve = json!({
            "type": "EventMemberLootReport",
            "body": { "content": { "EventName": "GVE" } }
        });
        assert_eq!(detect_mail_type(&gve), Some(MailType::EventMemberLootReport));

        for content in [json!({ "EventName": "OtherEvent" }), json!({}), json!({ "EventName": 1 })]
        {
            let mail = json!({
                "type": "EventMemberLootReport",
                "body": { "content": content }
            });
            assert_eq!(detect_mail_type(&mail), None);
        }
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
    fn detect_mail_type_matches_system_motte() {
        let payload = json!({
            "type": "System",
            "box": "Report",
            "body": {
                "subParam": 4,
                "subType": 11
            }
        });

        assert_eq!(detect_mail_type(&payload), Some(MailType::SystemBarbarianFort));
    }

    #[test]
    fn detect_mail_type_matches_system_kahar_treasure() {
        let payload = json!({
            "type": "System",
            "box": "SystemBox",
            "body": {
                "subParam": 11,
                "subType": 29
            }
        });

        assert_eq!(detect_mail_type(&payload), Some(MailType::SystemKaharTreasure));
    }

    #[test]
    fn detect_mail_type_matches_archived_system_barbarian_fort() {
        let payload = json!({
            "type": "System",
            "box": "Archive",
            "prevBox": "Report",
            "body": {
                "subParam": 1,
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
        let custom_battle_results = json!({
            "type": "Alliance",
            "body": { "type": 14, "param": 1 }
        });
        assert_eq!(
            detect_mail_type(&custom_battle_results),
            Some(MailType::AllianceAOOBattleResults)
        );

        let battle_info = json!({
            "type": "Alliance",
            "body": { "type": 61 }
        });
        assert_eq!(detect_mail_type(&battle_info), Some(MailType::AllianceAOOBattleInfo));

        let individual_results = json!({
            "type": "Alliance",
            "body": { "type": 62 }
        });
        assert_eq!(
            detect_mail_type(&individual_results),
            Some(MailType::AllianceAOOIndividualResults)
        );
    }

    #[test]
    fn detect_mail_type_matches_alliance_aoo_mail_in_any_mailbox() {
        let battle_results = json!({
            "type": "Alliance",
            "box": "Archive",
            "prevBox": "OtherBox",
            "body": { "type": 60, "param": 1 }
        });

        assert_eq!(detect_mail_type(&battle_results), Some(MailType::AllianceAOOBattleResults));
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
    fn detect_mail_type_rejects_alliance_mail_without_body() {
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
    fn detect_mail_type_checks_alliance_type_before_aoo_body() {
        let payload = json!({
            "type": "Battle",
            "body": { "type": 60, "param": 1 }
        });

        assert_eq!(detect_mail_type(&payload), Some(MailType::Battle));
    }

    #[test]
    fn detect_mail_type_matches_system_subtypes_without_requiring_box() {
        let payload = json!({
            "type": "System",
            "box": "AllianceBox",
            "body": { "subType": 11, "subParam": 1 }
        });

        assert_eq!(detect_mail_type(&payload), Some(MailType::SystemBarbarianFort));
    }

    #[test]
    fn detect_mail_type_rejects_partial_system_matches() {
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
        assert_eq!(
            MailType::EventMemberLootReport.collection_name(),
            "mails_eventmemberlootreport"
        );
        assert_eq!(MailType::Rss.collection_name(), "mails_rss");
        assert_eq!(MailType::SystemBarbarianFort.collection_name(), "mails_system_barbarianfort");
        assert_eq!(MailType::SystemKaharTreasure.collection_name(), "mails_system_kahartreasure");
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
    }

    #[cfg(feature = "processors")]
    #[test]
    fn process_mail_dispatches_system_kahar_treasure() {
        let payload = json!({
            "type": "System",
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "attachments": [
                {
                    "loot": [
                        { "Type": 1, "SubType": 9, "Value": 45000 }
                    ]
                }
            ],
            "body": {
                "subParam": 11,
                "subType": 29
            }
        });

        let processed =
            process_mail(MailType::SystemKaharTreasure, &payload).expect("process kahar treasure");
        let metadata = processed.sections().get("metadata").expect("metadata section");
        let loot = processed
            .sections()
            .get("loot")
            .and_then(rokbattles_mail_sdk::Section::array)
            .expect("loot section");

        assert_eq!(metadata.fields()["mail_id"], json!("mail-1"));
        assert_eq!(loot[0], json!({"type": 1, "sub_type": 9, "value": 45000}));
    }

    #[cfg(feature = "processors")]
    #[test]
    fn process_mail_dispatches_event_member_loot_report() {
        let payload = json!({
            "type": "EventMemberLootReport",
            "id": "mail-1",
            "time": 1234,
            "receiver": "player-1",
            "serverId": 55,
            "body": { "content": {
                "EventName": "GVE",
                "subTitle": "Bladefist Andaal Has Been Defeated",
                "infos": [{
                    "playerId": 7,
                    "name": "Player",
                    "avatar": null,
                    "loots": [{ "Type": 2, "SubType": 3, "Value": 4 }]
                }]
            }}
        });
        let processed =
            process_mail(MailType::EventMemberLootReport, &payload).expect("process GVE report");
        assert_eq!(processed.sections()["boss"].fields()["id"], json!(30001));
        assert_eq!(
            processed.sections()["participants"].array().expect("participants")[0]["player_id"],
            json!(7)
        );
    }
}
