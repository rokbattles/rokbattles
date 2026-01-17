use crate::{
    context::MailContext,
    helpers::{
        find_self_content_root, find_self_snapshot_section, get_or_insert_object_map,
        map_insert_f64_if_absent, map_insert_i64_if_absent, map_insert_str_if_absent, parse_f64,
    },
};
use mail_processor_sdk::Resolver;
use serde_json::Value;
use std::convert::Infallible;

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

    fn value_truthy(v: &Value) -> bool {
        match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => n
                .as_i64()
                .map(|x| x != 0)
                .or_else(|| n.as_u64().map(|x| x != 0))
                .or_else(|| n.as_f64().map(|x| x != 0.0))
                .unwrap_or(false),
            Value::String(s) => !s.trim().is_empty(),
            Value::Array(values) => values.iter().any(Self::value_truthy),
            Value::Object(map) => map.values().any(Self::value_truthy),
        }
    }

    fn has_strife_titan(v: &Value) -> bool {
        match v {
            Value::Object(map) => {
                if let Some(titan) = map.get("Titan")
                    && Self::value_truthy(titan)
                {
                    return true;
                }
                map.values().any(Self::has_strife_titan)
            }
            Value::Array(values) => values.iter().any(Self::has_strife_titan),
            _ => false,
        }
    }

    fn parse_i128(v: Option<&Value>) -> Option<i128> {
        match v {
            Some(Value::Number(n)) => n.as_i64().map(|x| x as i128),
            Some(Value::String(s)) => s.trim().parse::<i128>().ok(),
            _ => None,
        }
    }

    fn normalize_epoch_seconds(n: i128) -> Option<i64> {
        // 1e12 -> ms, 1e15 -> µs
        let abs = n.abs();
        let secs = if abs >= 1_000_000_000_000_000 {
            n / 1_000_000 // microseconds
        } else if abs >= 1_000_000_000_000 {
            n / 1_000 // milliseconds
        } else {
            n // seconds
        };
        i64::try_from(secs).ok()
    }

    fn parse_epoch_seconds(v: Option<&Value>) -> Option<i64> {
        Self::parse_i128(v).and_then(Self::normalize_epoch_seconds)
    }

    fn find_epoch_in_group(group: &[Value], key: &str) -> Option<i64> {
        group.iter().find_map(|s| {
            Self::parse_epoch_seconds(s.get(key)).or_else(|| {
                s.get("body")
                    .and_then(|b| Self::parse_epoch_seconds(b.get(key)))
            })
        })
    }

    fn first_epoch_geq(sections: &[Value], key: &str, min: i64) -> Option<i64> {
        sections.iter().find_map(|s| {
            if let Some(x) = Self::parse_epoch_seconds(s.get(key)).filter(|&x| x >= min) {
                Some(x)
            } else {
                s.get("body")
                    .and_then(|b| Self::parse_epoch_seconds(b.get(key)).filter(|&x| x >= min))
            }
        })
    }

    fn first_small_tickstart_in_sections(sections: &[Value]) -> Option<i64> {
        sections.iter().find_map(|s| {
            s.get("TickStart")
                .and_then(Value::as_i64)
                .or_else(|| {
                    s.get("Bts")
                        .and_then(Value::as_i64)
                        .filter(|&b| b < 1_000_000_000)
                })
                .or_else(|| {
                    s.get("Attacks").and_then(|a| {
                        a.get("TickStart")
                            .and_then(Value::as_i64)
                            .or_else(|| a.get("Bts").and_then(Value::as_i64))
                    })
                })
                .filter(|&b| b < 1_000_000_000)
                .or_else(|| {
                    s.get("body")
                        .and_then(|b| b.get("Bts").and_then(Value::as_i64))
                        .filter(|&b| b < 1_000_000_000)
                })
        })
    }

    fn find_small_tick_pair(group: &[Value]) -> Option<(i64, i64)> {
        // direct (ts, ets)
        if let Some(pair) = group.iter().find_map(|s| {
            let ts = s.get("TickStart").and_then(Value::as_i64).or_else(|| {
                s.get("Bts")
                    .and_then(Value::as_i64)
                    .filter(|&b| b < 1_000_000_000)
            });
            let ets = s
                .get("Ets")
                .and_then(Value::as_i64)
                .filter(|&e| e < 1_000_000_000);
            match (ts, ets) {
                (Some(ts), Some(ets)) if ets >= ts => Some((ts, ets)),
                _ => None,
            }
        }) {
            return Some(pair);
        }

        // TickStart + (T-1) derived
        group.iter().find_map(|s| {
            let ts = s.get("TickStart").and_then(Value::as_i64)?;
            let t = s
                .get("T")
                .and_then(Value::as_i64)
                .filter(|&t| t < 1_000_000_000)?;
            (t > ts).then_some((ts, ts + (t - ts - 1)))
        })
    }
}

impl Resolver<MailContext<'_>, Value> for MetadataResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, mail: &mut Value) -> Result<(), Self::Error> {
        let meta = get_or_insert_object_map(mail, "metadata");

        let sections = ctx.sections;
        let group = ctx.group;

        // attack id
        meta.entry("attack_id")
            .or_insert_with(|| Value::String(ctx.attack_id.to_string()));

        // email basics
        if let Some(g0) = sections.first() {
            map_insert_str_if_absent(meta, "email_id", g0.get("id").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_type", g0.get("type").and_then(Value::as_str));
            map_insert_i64_if_absent(
                meta,
                "email_time",
                Self::parse_i128(g0.get("time")).and_then(|n| i64::try_from(n).ok()),
            );
        }

        if meta.get("email_id").is_none() {
            let id = sections
                .iter()
                .find_map(|s| s.get("id").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_id", id);
        }

        if meta.get("email_type").is_none() {
            let email_type = sections
                .iter()
                .find_map(|s| s.get("type").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_type", email_type);
        }

        if meta.get("email_time").is_none() {
            let email_time = sections
                .iter()
                .find_map(|s| Self::parse_i128(s.get("time")).and_then(|n| i64::try_from(n).ok()));
            map_insert_i64_if_absent(meta, "email_time", email_time);
        }

        if meta.get("server_id").is_none() {
            let server_id = sections
                .iter()
                .find_map(|s| s.get("serverId").and_then(Value::as_i64));
            map_insert_i64_if_absent(meta, "server_id", server_id);
        }

        // email role
        let stats_block = sections.iter().find(|s| {
            s.get("STs").is_some()
                || s.get("Role").is_some()
                || s.get("body")
                    .and_then(|b| b.get("STs").or_else(|| b.get("Role")))
                    .is_some()
        });

        if let Some(role) = stats_block
            .and_then(|s| {
                s.get("Role")
                    .or_else(|| s.get("body").and_then(|b| b.get("Role")))
            })
            .and_then(Value::as_str)
        {
            map_insert_str_if_absent(meta, "email_role", Some(role));
        }

        // kvk
        let legacy_is_kvk = stats_block.and_then(|s| {
            s.get("isConquerSeason")
                .or_else(|| s.get("body").and_then(|b| b.get("isConquerSeason")))
                .and_then(Value::as_bool)
        });

        let computed_is_kvk = {
            let server_id = sections
                .first()
                .and_then(|s| s.get("serverId").and_then(Value::as_i64))
                .or_else(|| {
                    sections
                        .iter()
                        .find_map(|s| s.get("GsId").and_then(Value::as_i64))
                });

            let self_snap = find_self_snapshot_section(sections);
            let self_cosid = self_snap
                .and_then(|s| s.get("COSId").and_then(Value::as_i64))
                .or_else(|| {
                    find_self_content_root(sections)
                        .and_then(|r| r.pointer("/SelfChar/COSId").and_then(Value::as_i64))
                });

            match (server_id, self_cosid) {
                // If server and city owner differ we treat it as kvk; otherwise defer to legacy flag.
                (Some(sid), Some(cos)) if sid != 0 && cos != 0 => Some((sid != cos) as i32),
                _ => None,
            }
        };

        let is_kvk_val = legacy_is_kvk
            .map(|b| if b { 1 } else { 0 })
            .or(computed_is_kvk)
            .unwrap_or(0);
        meta.entry("is_kvk").or_insert(Value::from(is_kvk_val));

        if meta.get("is_strife").is_none() {
            let is_strife = sections.iter().any(Self::has_strife_titan);
            meta.insert(
                "is_strife".to_string(),
                Value::from(if is_strife { 1 } else { 0 }),
            );
        }

        // start and end time
        let base_epoch = Self::first_epoch_geq(sections, "Bts", 1_000_000_000)
            .or_else(|| {
                sections
                    .iter()
                    .find_map(|s| Self::parse_epoch_seconds(s.get("Bts")))
            })
            .unwrap_or(0);
        let base_small = Self::first_small_tickstart_in_sections(sections).unwrap_or(0);

        let (ts_small, ets_small) = if let Some(pair) = Self::find_small_tick_pair(group) {
            pair
        } else {
            // Mix and match epoch-based and tick-based timestamps so we can always compute start/end.
            // TODO this is the incorrect way of computing, but we'll fix it at a later date, it
            //  produces nearly the same output as intended
            let gba = Self::find_epoch_in_group(group, "Bts").unwrap_or(base_epoch);
            let gea = Self::find_epoch_in_group(group, "Ets").unwrap_or(base_epoch);
            let ts_small = if gba < 1_000_000_000 {
                gba
            } else {
                gba - base_epoch + base_small
            };
            let ets_small = if gea < 1_000_000_000 {
                gea
            } else {
                gea - base_epoch + base_small
            };
            (ts_small, ets_small)
        };

        let start_date = base_epoch + (ts_small - base_small);
        let end_date = base_epoch + (ets_small - base_small);
        map_insert_i64_if_absent(meta, "start_date", Some(start_date));
        map_insert_i64_if_absent(meta, "end_date", Some(end_date));

        // position
        if let Some(pos) = group.iter().find_map(|s| {
            s.get("Pos")
                .or_else(|| s.get("Attacks").and_then(|a| a.get("Pos")))
        }) {
            map_insert_f64_if_absent(meta, "pos_x", parse_f64(pos.get("X")));
            map_insert_f64_if_absent(meta, "pos_y", parse_f64(pos.get("Y")));
        } else if let Some(attacks_obj) = sections
            .iter()
            .find_map(|s| s.get("Attacks").filter(|a| a.get(ctx.attack_id).is_some()))
            .and_then(|a| a.get("Pos"))
        {
            map_insert_f64_if_absent(meta, "pos_x", parse_f64(attacks_obj.get("X")));
            map_insert_f64_if_absent(meta, "pos_y", parse_f64(attacks_obj.get("Y")));
        }

        // email receiver
        if meta.get("email_receiver").is_none() {
            let receiver = sections.iter().find_map(|section| {
                section.get("receiver").and_then(|value| match value {
                    Value::String(s) => Some(s.to_owned()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
            });
            if let Some(receiver) = receiver {
                meta.insert("email_receiver".to_string(), Value::String(receiver));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MetadataResolver;
    use crate::context::MailContext;
    use mail_processor_sdk::Resolver;
    use serde_json::{Value, json};

    fn resolve_metadata(sections: &[Value]) -> Value {
        let mut mail = json!({ "metadata": {} });
        let ctx = MailContext::new(sections, sections, "0");
        MetadataResolver::new()
            .resolve(&ctx, &mut mail)
            .expect("resolve metadata");
        mail["metadata"].clone()
    }

    #[test]
    fn metadata_sets_is_strife_for_supreme_strife_reports() {
        let sections = vec![json!({
            "body": {
                "content": {
                    "SelfChar": {
                        "PId": 1,
                        "Titan": {
                            "Pioneer": true,
                            "Round": 1,
                            "BattleId": "21009_1768262400898",
                            "TeamId": 2
                        }
                    }
                }
            }
        })];
        let metadata = resolve_metadata(&sections);
        assert_eq!(metadata.get("is_strife").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn metadata_sets_is_strife_for_non_pioneer_supreme_strife_reports() {
        let sections = vec![json!({
            "body": {
                "content": {
                    "SelfChar": {
                        "PId": 1,
                        "Titan": {
                            "Pioneer": false,
                            "Round": 1,
                            "BattleId": "21009_1768262400898",
                            "TeamId": 2
                        }
                    }
                }
            }
        })];
        let metadata = resolve_metadata(&sections);
        assert_eq!(metadata.get("is_strife").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn metadata_clears_is_strife_for_non_strife_reports() {
        let sections = vec![json!({
            "body": {
                "content": {
                    "SelfChar": {
                        "PId": 2,
                        "Titan": {
                            "Round": 0,
                            "BattleId": "",
                            "TeamId": 0
                        }
                    }
                }
            }
        })];
        let metadata = resolve_metadata(&sections);
        assert_eq!(metadata.get("is_strife").and_then(Value::as_i64), Some(0));
    }
}
