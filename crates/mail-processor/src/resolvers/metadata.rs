use crate::{
    helpers::{
        find_self_content_root, find_self_snapshot_section, get_or_insert_object,
        map_insert_f64_if_absent, map_insert_i64_if_absent, map_insert_str_if_absent, parse_f64,
    },
    resolvers::{Resolver, ResolverContext},
};
use serde_json::Value;

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

impl Resolver for MetadataResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        let sections = ctx.sections;
        let group = ctx.group;

        // attack id
        meta.entry("attack_id")
            .or_insert_with(|| Value::String(ctx.attack_id.to_string()));

        // email basics
        if let Some(g0) = sections.first() {
            map_insert_str_if_absent(meta, "email_id", g0.get("id").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_type", g0.get("type").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_box", g0.get("box").and_then(Value::as_str));
            map_insert_i64_if_absent(
                meta,
                "email_time",
                Self::parse_i128(g0.get("time")).and_then(|n| i64::try_from(n).ok()),
            );
        }

        // email role
        let stats_block = sections
            .iter()
            .find(|s| s.get("STs").is_some() || s.get("Role").is_some());

        if let Some(role) = stats_block
            .and_then(|s| s.get("Role"))
            .and_then(Value::as_str)
        {
            map_insert_str_if_absent(meta, "email_role", Some(role));
        }

        // kvk
        let is_kvk = stats_block
            .and_then(|s| s.get("isConquerSeason").and_then(Value::as_bool))
            .unwrap_or(false);
        meta.entry("is_kvk").or_insert(Value::from(is_kvk as i32));

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
            let gba = Self::find_epoch_in_group(group, "Bts").unwrap_or(base_epoch);
            let gea = Self::find_epoch_in_group(group, "Ets").unwrap_or(base_epoch);
            (gba - base_epoch + base_small, gea - base_epoch + base_small)
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

        // player count
        if let Some(sts) = stats_block
            .and_then(|s| s.get("STs"))
            .and_then(Value::as_object)
        {
            let cnt = sts.keys().filter(|k| k.as_str() != "-2").count() as i32;
            meta.entry("players").or_insert(Value::from(cnt));
        }

        // email receiver
        let self_snap = find_self_snapshot_section(sections).unwrap_or(&Value::Null);
        let self_body = find_self_content_root(sections).unwrap_or(&Value::Null);
        if let Some(pid) = self_snap
            .get("PId")
            .and_then(Value::as_i64)
            .or_else(|| self_body.pointer("/SelfChar/PId").and_then(Value::as_i64))
            && pid != 0
        {
            meta.entry("email_receiver")
                .or_insert(Value::String(pid.to_string()));
        }

        Ok(())
    }
}
