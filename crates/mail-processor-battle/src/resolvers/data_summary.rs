use std::convert::Infallible;

use mail_processor_sdk::Resolver;
use serde_json::{Map, Value};

use crate::context::MailContext;
use crate::structures::{BattleMail, DataSummary};

/// Resolves aggregated SOv/OOv summary data from battle report sections.
pub struct DataSummaryResolver;

impl Default for DataSummaryResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSummaryResolver {
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

    fn find_first_object_for_key<'a>(
        sections: &'a [Value],
        key: &str,
    ) -> Option<&'a Map<String, Value>> {
        sections
            .iter()
            .find_map(|section| section.get(key).and_then(Value::as_object))
    }

    /// Copies overview stats into the summary, choosing sender or opponent fields.
    fn fill_summary(summary: &mut DataSummary, src: &Map<String, Value>, is_sender: bool) {
        let kill_points = src.get("KillScore").and_then(Self::parse_i64);
        let severely_wounded = src.get("BadHurt").and_then(Self::parse_i64);
        let slightly_wounded = src.get("Hurt").and_then(Self::parse_i64);
        let troop_units = src.get("Max").and_then(Self::parse_i64);
        let remaining = src.get("Cnt").and_then(Self::parse_i64);
        let dead = src.get("Dead").and_then(Self::parse_i64);

        if is_sender {
            summary.sender_kill_points = kill_points;
            summary.sender_severely_wounded = severely_wounded;
            summary.sender_slightly_wounded = slightly_wounded;
            summary.sender_troop_units = troop_units;
            summary.sender_remaining = remaining;
            summary.sender_dead = dead;
        } else {
            summary.opponent_kill_points = kill_points;
            summary.opponent_severely_wounded = severely_wounded;
            summary.opponent_slightly_wounded = slightly_wounded;
            summary.opponent_troop_units = troop_units;
            summary.opponent_remaining = remaining;
            summary.opponent_dead = dead;
        }
    }
}

impl Resolver<MailContext<'_>, BattleMail> for DataSummaryResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, output: &mut BattleMail) -> Result<(), Self::Error> {
        let sections = ctx.sections;
        let sender = Self::find_first_object_for_key(sections, "SOv");
        let opponent = Self::find_first_object_for_key(sections, "OOv");

        if sender.is_none() && opponent.is_none() {
            return Ok(());
        }

        let mut summary = DataSummary::default();

        if let Some(src) = sender {
            Self::fill_summary(&mut summary, src, true);
        }
        if let Some(src) = opponent {
            Self::fill_summary(&mut summary, src, false);
        }

        output.data_summary = Some(summary);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::DataSummaryResolver;
    use crate::context::MailContext;
    use crate::structures::BattleMail;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_summary(sections: Vec<Value>) -> BattleMail {
        let ctx = MailContext::new(&sections);
        let mut output = BattleMail::default();
        let resolver = DataSummaryResolver::new();

        resolver
            .resolve(&ctx, &mut output)
            .expect("resolve data summary");

        output
    }

    #[test]
    fn data_summary_resolver_leaves_empty_without_overview() {
        let sections = vec![json!({
            "id": "mail-14",
            "time": 1111,
            "serverId": 1804
        })];

        let output = resolve_summary(sections);

        assert!(output.data_summary.is_none());
    }

    #[test]
    fn data_summary_resolver_loads_from_same_section() {
        let sections = vec![json!({
            "SOv": {
                "KillScore": 307090,
                "BadHurt": 14528,
                "Max": 230000,
                "Hurt": 85754,
                "Cnt": 129718,
                "Dead": 0
            },
            "OOv": {
                "KillScore": 290560,
                "BadHurt": 30709,
                "Max": 418491,
                "Hurt": 166508,
                "Cnt": 17659,
                "Dead": 0
            }
        })];

        let output = resolve_summary(sections);
        let summary = output.data_summary.expect("data summary");

        assert_eq!(summary.sender_kill_points, Some(307090));
        assert_eq!(summary.sender_severely_wounded, Some(14528));
        assert_eq!(summary.sender_slightly_wounded, Some(85754));
        assert_eq!(summary.sender_troop_units, Some(230000));
        assert_eq!(summary.sender_remaining, Some(129718));
        assert_eq!(summary.sender_dead, Some(0));
        assert_eq!(summary.opponent_kill_points, Some(290560));
        assert_eq!(summary.opponent_severely_wounded, Some(30709));
        assert_eq!(summary.opponent_slightly_wounded, Some(166508));
        assert_eq!(summary.opponent_troop_units, Some(418491));
        assert_eq!(summary.opponent_remaining, Some(17659));
        assert_eq!(summary.opponent_dead, Some(0));
    }

    #[test]
    fn data_summary_resolver_loads_from_separate_sections() {
        let sections = vec![
            json!({
                "OOv": {
                    "KillScore": 5212360,
                    "BadHurt": 859533,
                    "Max": 6804444,
                    "Hurt": 3279249,
                    "Cnt": 116032,
                    "Dead": 0
                }
            }),
            json!({
                "SOv": {
                    "KillScore": 16991393,
                    "BadHurt": 272818,
                    "Max": 2730000,
                    "Hurt": 1400260,
                    "Cnt": 1056922,
                    "Dead": 0
                }
            }),
        ];

        let output = resolve_summary(sections);
        let summary = output.data_summary.expect("data summary");

        assert_eq!(summary.sender_kill_points, Some(16991393));
        assert_eq!(summary.sender_severely_wounded, Some(272818));
        assert_eq!(summary.sender_slightly_wounded, Some(1400260));
        assert_eq!(summary.sender_troop_units, Some(2730000));
        assert_eq!(summary.sender_remaining, Some(1056922));
        assert_eq!(summary.sender_dead, Some(0));
        assert_eq!(summary.opponent_kill_points, Some(5212360));
        assert_eq!(summary.opponent_severely_wounded, Some(859533));
        assert_eq!(summary.opponent_slightly_wounded, Some(3279249));
        assert_eq!(summary.opponent_troop_units, Some(6804444));
        assert_eq!(summary.opponent_remaining, Some(116032));
        assert_eq!(summary.opponent_dead, Some(0));
    }
}
