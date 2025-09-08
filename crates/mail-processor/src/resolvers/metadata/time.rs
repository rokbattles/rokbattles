use crate::helpers::{get_or_insert_object, map_insert_i64_if_absent};
use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct TimeResolver;

impl Default for TimeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeResolver {
    pub fn new() -> Self {
        Self
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

impl Resolver for TimeResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        let sections = ctx.sections;
        let group = ctx.group;

        // compute base epoch and small tick base
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

        Ok(())
    }
}
