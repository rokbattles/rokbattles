use crate::helpers::get_or_insert_object;
use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct PlayersResolver;

impl Default for PlayersResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayersResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for PlayersResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        let sections = ctx.sections;

        let stats_block = sections.iter().find(|s| {
            s.get("STs").is_some()
                || s.get("Role").is_some()
                || s.get("body")
                    .and_then(|b| b.get("STs").or_else(|| b.get("Role")))
                    .is_some()
        });

        if let Some(sts) = stats_block
            .and_then(|s| {
                s.get("STs")
                    .or_else(|| s.get("body").and_then(|b| b.get("STs")))
            })
            .and_then(Value::as_object)
        {
            let cnt = sts.keys().filter(|k| k.as_str() != "-2").count() as i32;
            meta.entry("players").or_insert(Value::from(cnt));
        }

        Ok(())
    }
}
