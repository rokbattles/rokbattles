use crate::helpers::{get_or_insert_object, map_insert_f64_if_absent, parse_f64};
use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct PositionResolver;

impl Default for PositionResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for PositionResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        let sections = ctx.sections;
        let group = ctx.group;

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

        Ok(())
    }
}
