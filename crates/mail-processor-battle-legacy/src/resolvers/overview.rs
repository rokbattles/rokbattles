use crate::{
    context::MailContext,
    helpers::{get_or_insert_object_map, map_get_i64, map_put_i64_with_prefix},
};
use mail_processor_sdk_legacy::Resolver;
use serde_json::{Map, Value};
use std::convert::Infallible;

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

    fn copy_overview_stats(dst: &mut Map<String, Value>, src: &Map<String, Value>, prefix: &str) {
        map_put_i64_with_prefix(dst, prefix, "kill_score", map_get_i64(src, "KillScore"));
        map_put_i64_with_prefix(dst, prefix, "max", map_get_i64(src, "Max"));
        map_put_i64_with_prefix(dst, prefix, "severely_wounded", map_get_i64(src, "BadHurt"));
        map_put_i64_with_prefix(
            dst,
            prefix,
            "death",
            map_get_i64(src, "Dead").or_else(|| map_get_i64(src, "Death")),
        );
        map_put_i64_with_prefix(dst, prefix, "wounded", map_get_i64(src, "Hurt"));
        map_put_i64_with_prefix(dst, prefix, "remaining", map_get_i64(src, "Cnt"));
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

impl Resolver<MailContext<'_>, Value> for OverviewResolver {
    type Error = Infallible;

    fn resolve(&self, ctx: &MailContext<'_>, mail: &mut Value) -> Result<(), Self::Error> {
        let self_overview = Self::find_first_object_for_key(ctx.group, "SOv")
            .or_else(|| Self::find_first_object_for_key(ctx.sections, "SOv"));
        let enemy_overview = Self::find_first_object_for_key(ctx.group, "OOv")
            .or_else(|| Self::find_first_object_for_key(ctx.sections, "OOv"));

        if self_overview.is_none() && enemy_overview.is_none() {
            return Ok(());
        }

        let dst = get_or_insert_object_map(mail, "overview");

        if let Some(m) = self_overview {
            Self::copy_overview_stats(dst, m, "");
        }
        if let Some(m) = enemy_overview {
            Self::copy_overview_stats(dst, m, "enemy_");
        }

        Ok(())
    }
}
