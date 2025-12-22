use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::structures::{
    BattleAssistUnits, BattleCommander, BattleCommanders, BattleEvent, BattleMail, BattleSampling,
    BattleTrends,
};

/// Resolves sampling and event trend data from battle report sections.
pub struct BattleTrendsResolver;

impl Default for BattleTrendsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleTrendsResolver {
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

    fn find_samples_chain(sections: &[Value]) -> Option<Vec<BattleSampling>> {
        for (index, section) in sections.iter().enumerate() {
            let mut ancestors = Vec::new();
            if let Some(mut chain) = Self::find_samples_chain_in_value(section, &mut ancestors) {
                Self::append_following_sections(&mut chain, sections, index);
                return Some(chain);
            }
        }

        None
    }

    fn find_events_chain(sections: &[Value]) -> Option<Vec<BattleEvent>> {
        for (index, section) in sections.iter().enumerate() {
            let mut ancestors = Vec::new();
            if let Some(mut chain) = Self::find_events_chain_in_value(section, &mut ancestors) {
                Self::append_following_event_sections(&mut chain, sections, index);
                return Some(chain);
            }
        }

        None
    }

    fn find_samples_chain_in_value<'a>(
        value: &'a Value,
        ancestors: &mut Vec<&'a Map<String, Value>>,
    ) -> Option<Vec<BattleSampling>> {
        match value {
            Value::Object(map) => {
                if let Some(Value::Object(samples)) = map.get("Samples") {
                    let mut chain = Vec::new();
                    let sample = Self::to_sampling(samples);
                    if sample.count.is_some() || sample.tick.is_some() {
                        chain.push(sample);
                    }

                    // The Samples object bleeds into its parent and beyond.
                    let parent_sample = Self::to_sampling(map);
                    if parent_sample.count.is_some() || parent_sample.tick.is_some() {
                        chain.push(parent_sample);
                    }

                    for ancestor in ancestors.iter().rev() {
                        let ancestor_sample = Self::to_sampling(ancestor);
                        if ancestor_sample.count.is_some() || ancestor_sample.tick.is_some() {
                            chain.push(ancestor_sample);
                        }
                    }

                    return Some(chain);
                }

                ancestors.push(map);

                for nested in map.values() {
                    if let Some(found) = Self::find_samples_chain_in_value(nested, ancestors) {
                        return Some(found);
                    }
                }

                ancestors.pop();

                None
            }
            Value::Array(values) => {
                for nested in values {
                    if let Some(found) = Self::find_samples_chain_in_value(nested, ancestors) {
                        return Some(found);
                    }
                }

                None
            }
            _ => None,
        }
    }

    fn find_events_chain_in_value<'a>(
        value: &'a Value,
        ancestors: &mut Vec<&'a Map<String, Value>>,
    ) -> Option<Vec<BattleEvent>> {
        match value {
            Value::Object(map) => {
                if let Some(Value::Object(events)) = map.get("Events") {
                    let mut chain = Vec::new();
                    let event = Self::to_event(events);
                    let has_event_data = event.r#type.is_some()
                        || event.tick.is_some()
                        || event.reinforcements.is_some();
                    // Empty Events objects appear alongside Samples; ignore them to avoid bleed.
                    if !has_event_data {
                        // Keep searching in case later sections contain real events.
                        // Do not include parent bleed when the Events object is empty.
                    } else {
                        chain.push(event);

                        // Events data bleeds into the parent and beyond.
                        let parent_event = Self::to_event(map);
                        if parent_event.r#type.is_some()
                            || parent_event.tick.is_some()
                            || parent_event.reinforcements.is_some()
                        {
                            chain.push(parent_event);
                        }

                        for ancestor in ancestors.iter().rev() {
                            let ancestor_event = Self::to_event(ancestor);
                            if ancestor_event.r#type.is_some()
                                || ancestor_event.tick.is_some()
                                || ancestor_event.reinforcements.is_some()
                            {
                                chain.push(ancestor_event);
                            }
                        }

                        return Some(chain);
                    }
                }

                ancestors.push(map);

                for nested in map.values() {
                    if let Some(found) = Self::find_events_chain_in_value(nested, ancestors) {
                        return Some(found);
                    }
                }

                ancestors.pop();

                None
            }
            Value::Array(values) => {
                for nested in values {
                    if let Some(found) = Self::find_events_chain_in_value(nested, ancestors) {
                        return Some(found);
                    }
                }

                None
            }
            _ => None,
        }
    }

    fn append_following_sections(
        chain: &mut Vec<BattleSampling>,
        sections: &[Value],
        start_index: usize,
    ) {
        // Sampling continues in top-level sections after the Samples container until a gap.
        for section in sections.iter().skip(start_index + 1) {
            let Some(map) = section.as_object() else {
                break;
            };

            let sample = Self::to_sampling(map);
            if sample.count.is_none() && sample.tick.is_none() {
                break;
            }

            chain.push(sample);
        }
    }

    fn append_following_event_sections(
        chain: &mut Vec<BattleEvent>,
        sections: &[Value],
        start_index: usize,
    ) {
        // Events continue in top-level sections after the Events container until a gap.
        for section in sections.iter().skip(start_index + 1) {
            let Some(map) = section.as_object() else {
                break;
            };

            let event = Self::to_event(map);
            if event.r#type.is_none() && event.tick.is_none() && event.reinforcements.is_none() {
                break;
            }

            chain.push(event);
        }
    }

    fn to_sampling(map: &Map<String, Value>) -> BattleSampling {
        BattleSampling {
            count: map.get("Cnt").and_then(Self::parse_i64),
            tick: map.get("T").and_then(Self::parse_i64),
        }
    }

    fn to_event(map: &Map<String, Value>) -> BattleEvent {
        BattleEvent {
            r#type: map.get("Et").and_then(Self::parse_i64),
            tick: map.get("T").and_then(Self::parse_i64),
            reinforcements: map
                .get("AssistUnits")
                .and_then(Value::as_object)
                .map(Self::to_assist_units),
        }
    }

    fn to_assist_units(map: &Map<String, Value>) -> BattleAssistUnits {
        let (avatar_url, frame_url) = Self::parse_avatar(map.get("Avatar"));

        let primary = Self::to_commander(map.get("HId"), map.get("HLv"));
        let secondary = Self::to_commander(map.get("HId2"), map.get("HLv2"));
        let commanders = match (primary, secondary) {
            (None, None) => None,
            (primary, secondary) => Some(BattleCommanders { primary, secondary }),
        };

        BattleAssistUnits {
            player_id: map.get("PId").and_then(Self::parse_i64),
            player_name: map.get("PName").and_then(Value::as_str).map(str::to_owned),
            avatar_url,
            frame_url,
            commanders,
        }
    }

    fn to_commander(
        id_value: Option<&Value>,
        level_value: Option<&Value>,
    ) -> Option<BattleCommander> {
        let id = id_value.and_then(Self::parse_i64);
        let level = level_value.and_then(Self::parse_i64);

        if id.is_none() && level.is_none() {
            return None;
        }

        Some(BattleCommander { id, level })
    }

    fn parse_avatar(value: Option<&Value>) -> (Option<String>, Option<String>) {
        let Some(value) = value else {
            return (None, None);
        };

        match value {
            Value::String(raw) => {
                if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
                    return (
                        parsed
                            .get("avatar")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        parsed
                            .get("avatarFrame")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    );
                }

                (None, None)
            }
            Value::Object(map) => (
                map.get("avatar").and_then(Value::as_str).map(str::to_owned),
                map.get("avatarFrame")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
            _ => (None, None),
        }
    }
}

impl Resolver<MailContext<'_>, BattleMail> for BattleTrendsResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let sampling = Self::find_samples_chain(ctx.sections).unwrap_or_default();
        let events = Self::find_events_chain(ctx.sections).unwrap_or_default();

        if sampling.is_empty() && events.is_empty() {
            return Ok(());
        }

        output.battle_trends = Some(BattleTrends { sampling, events });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BattleTrendsResolver;
    use crate::context::MailContext;
    use crate::structures::BattleMail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_trends(sections: Vec<Value>) -> BattleMail {
        let ctx = MailContext::new(&sections);
        let mut output = BattleMail::default();
        let resolver = BattleTrendsResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve battle trends");

        output
    }

    #[test]
    fn battle_trends_resolver_skips_when_missing_samples() {
        let sections = vec![json!({
            "id": "mail-1",
            "time": 123
        })];

        let output = resolve_trends(sections);

        assert!(output.battle_trends.is_none());
    }

    #[test]
    fn battle_trends_resolver_reads_samples_in_body() {
        let sections = vec![json!({
            "body": {
                "Samples": {
                    "Cnt": 42,
                    "T": 1337,
                    "Time": 123456
                }
            }
        })];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");
        let sampling = trends.sampling;

        assert_eq!(sampling.len(), 1);
        assert_eq!(sampling[0].count, Some(42));
        assert_eq!(sampling[0].tick, Some(1337));
    }

    #[test]
    fn battle_trends_resolver_collects_parent_bleed() {
        let sections = vec![json!({
            "Samples": {
                "Cnt": 7,
                "T": 99,
                "Time": 555
            },
            "Cnt": 500,
            "T": 600
        })];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");
        let sampling = trends.sampling;

        assert_eq!(sampling.len(), 2);
        assert_eq!(sampling[0].count, Some(7));
        assert_eq!(sampling[0].tick, Some(99));
        assert_eq!(sampling[1].count, Some(500));
        assert_eq!(sampling[1].tick, Some(600));
    }

    #[test]
    fn battle_trends_resolver_collects_bleed_across_levels() {
        let sections = vec![json!({
            "Cnt": 555,
            "T": 444,
            "body": {
                "Cnt": 777,
                "T": 666,
                "content": {
                    "Samples": {
                        "Cnt": 5,
                        "T": 55,
                        "Time": 123
                    },
                    "Cnt": 999,
                    "T": 888
                }
            }
        })];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");
        let sampling = trends.sampling;

        assert_eq!(sampling.len(), 4);
        assert_eq!(sampling[0].count, Some(5));
        assert_eq!(sampling[0].tick, Some(55));
        assert_eq!(sampling[1].count, Some(999));
        assert_eq!(sampling[1].tick, Some(888));
        assert_eq!(sampling[2].count, Some(777));
        assert_eq!(sampling[2].tick, Some(666));
        assert_eq!(sampling[3].count, Some(555));
        assert_eq!(sampling[3].tick, Some(444));
    }

    #[test]
    fn battle_trends_resolver_collects_following_sections_until_gap() {
        let sections = vec![
            json!({
                "Samples": {
                    "Cnt": 1,
                    "T": 10
                },
                "Cnt": 2,
                "T": 11
            }),
            json!({
                "Cnt": 3,
                "T": 12
            }),
            json!({
                "Cnt": 4,
                "T": 13
            }),
            json!({
                "Schema": 100
            }),
            json!({
                "Cnt": 5,
                "T": 14
            }),
        ];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");
        let sampling = trends.sampling;

        assert_eq!(sampling.len(), 4);
        assert_eq!(sampling[0].count, Some(1));
        assert_eq!(sampling[0].tick, Some(10));
        assert_eq!(sampling[1].count, Some(2));
        assert_eq!(sampling[1].tick, Some(11));
        assert_eq!(sampling[2].count, Some(3));
        assert_eq!(sampling[2].tick, Some(12));
        assert_eq!(sampling[3].count, Some(4));
        assert_eq!(sampling[3].tick, Some(13));
    }

    #[test]
    fn battle_trends_resolver_collects_events_with_bleed_and_following_sections() {
        let sections = vec![
            json!({
                "Events": {
                    "Et": 18,
                    "T": 100,
                    "AssistUnits": {
                        "PId": 1,
                        "PName": "HelperOne",
                        "Avatar": "{\"avatar\":\"http://a\",\"avatarFrame\":\"http://f\"}",
                        "HId": 10,
                        "HLv": 60,
                        "HId2": 11,
                        "HLv2": 30
                    }
                },
                "Et": 26,
                "T": 101,
                "AssistUnits": {
                    "PId": 2,
                    "PName": "HelperTwo"
                }
            }),
            json!({
                "Et": 18,
                "T": 102
            }),
            json!({
                "Schema": 1
            }),
            json!({
                "Et": 26,
                "T": 103
            }),
        ];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");
        let events = trends.events;

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].r#type, Some(18));
        assert_eq!(events[0].tick, Some(100));
        let assist = events[0].reinforcements.as_ref().expect("assist units");
        assert_eq!(assist.player_id, Some(1));
        assert_eq!(assist.player_name.as_deref(), Some("HelperOne"));
        assert_eq!(assist.avatar_url.as_deref(), Some("http://a"));
        assert_eq!(assist.frame_url.as_deref(), Some("http://f"));
        let commanders = assist.commanders.as_ref().expect("commanders");
        let primary = commanders.primary.as_ref().expect("primary commander");
        let secondary = commanders.secondary.as_ref().expect("secondary commander");
        assert_eq!(primary.id, Some(10));
        assert_eq!(primary.level, Some(60));
        assert_eq!(secondary.id, Some(11));
        assert_eq!(secondary.level, Some(30));

        assert_eq!(events[1].r#type, Some(26));
        assert_eq!(events[1].tick, Some(101));
        let assist = events[1].reinforcements.as_ref().expect("assist units");
        assert_eq!(assist.player_id, Some(2));
        assert_eq!(assist.player_name.as_deref(), Some("HelperTwo"));
        assert!(assist.avatar_url.is_none());
        assert!(assist.frame_url.is_none());
        assert!(assist.commanders.is_none());

        assert_eq!(events[2].r#type, Some(18));
        assert_eq!(events[2].tick, Some(102));
        assert!(events[2].reinforcements.is_none());
    }

    #[test]
    fn battle_trends_resolver_ignores_empty_events_near_samples() {
        let sections = vec![
            json!({
                "Events": {},
                "Samples": {
                    "Cnt": 438268,
                    "T": 1319563
                },
                "Cnt": 422054,
                "T": 1319564
            }),
            json!({
                "Cnt": 419813,
                "T": 1319579
            }),
            json!({
                "Schema": 810
            }),
        ];

        let output = resolve_trends(sections);
        let trends = output.battle_trends.expect("battle trends");

        assert_eq!(trends.sampling.len(), 3);
        assert!(trends.events.is_empty());
    }
}
