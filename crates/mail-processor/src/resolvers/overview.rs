use crate::resolvers::{Resolver, ResolverContext};
use serde_json::{Map, Value};

pub struct OverviewResolver;

impl Default for OverviewResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl OverviewResolver {
    pub fn new() -> Self {
        Self
    }

    fn get_i64_field(m: &Map<String, Value>, key: &str) -> Option<i64> {
        m.get(key).and_then(Value::as_i64)
    }

    fn insert_i64_with_prefix(
        dst: &mut Map<String, Value>,
        prefix: &str,
        name: &str,
        val: Option<i64>,
    ) {
        if let Some(v) = val {
            dst.insert(format!("{prefix}{name}"), Value::from(v));
        }
    }

    fn copy_overview_stats(dst: &mut Map<String, Value>, src: &Map<String, Value>, prefix: &str) {
        Self::insert_i64_with_prefix(
            dst,
            prefix,
            "kill_score",
            Self::get_i64_field(src, "KillScore"),
        );
        Self::insert_i64_with_prefix(dst, prefix, "max", Self::get_i64_field(src, "Max"));
        Self::insert_i64_with_prefix(
            dst,
            prefix,
            "severely_wounded",
            Self::get_i64_field(src, "BadHurt"),
        );
        Self::insert_i64_with_prefix(
            dst,
            prefix,
            "death",
            Self::get_i64_field(src, "Dead").or_else(|| Self::get_i64_field(src, "Death")),
        );
        Self::insert_i64_with_prefix(dst, prefix, "wounded", Self::get_i64_field(src, "Hurt"));
        Self::insert_i64_with_prefix(dst, prefix, "remaining", Self::get_i64_field(src, "Cnt"));
    }

    fn find_first_object_for_key<'a>(
        group: &'a [Value],
        key: &str,
    ) -> Option<&'a Map<String, Value>> {
        group
            .iter()
            .find_map(|s| s.get(key).and_then(Value::as_object))
    }
}

impl Resolver for OverviewResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let self_overview = Self::find_first_object_for_key(ctx.group, "SOv")
            .or_else(|| Self::find_first_object_for_key(ctx.sections, "SOv"));
        let enemy_overview = Self::find_first_object_for_key(ctx.group, "OOv")
            .or_else(|| Self::find_first_object_for_key(ctx.sections, "OOv"));

        if self_overview.is_none() && enemy_overview.is_none() {
            return Ok(());
        }

        let root = mail.as_object_mut().expect("mail root must be an object");
        let target = root
            .entry("overview".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if target.is_null() {
            *target = Value::Object(Map::new());
        }
        let dst = target
            .as_object_mut()
            .expect("overview must be an object when present");

        if let Some(m) = self_overview {
            Self::copy_overview_stats(dst, m, "");
        }
        if let Some(m) = enemy_overview {
            Self::copy_overview_stats(dst, m, "enemy_");
        }

        Ok(())
    }
}
