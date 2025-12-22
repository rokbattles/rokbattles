use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::structures::{BattleMail, BattleSampling, BattleTrends};

/// Resolves sampling trend data from the "Samples" object in battle report sections.
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

    fn to_sampling(map: &Map<String, Value>) -> BattleSampling {
        BattleSampling {
            count: map.get("Cnt").and_then(Self::parse_i64),
            tick: map.get("T").and_then(Self::parse_i64),
        }
    }
}

impl Resolver<MailContext<'_>, BattleMail> for BattleTrendsResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let Some(sampling) = Self::find_samples_chain(ctx.sections) else {
            return Ok(());
        };

        if sampling.is_empty() {
            return Ok(());
        }

        output.battle_trends = Some(BattleTrends { sampling });

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
}
