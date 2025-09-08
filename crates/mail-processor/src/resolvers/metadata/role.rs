use crate::helpers::{get_or_insert_object, map_insert_str_if_absent};
use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct RoleSeasonResolver;

impl Default for RoleSeasonResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleSeasonResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Resolver for RoleSeasonResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        let sections = ctx.sections;

        // locate stats block
        let stats_block = sections.iter().find(|s| {
            s.get("STs").is_some()
                || s.get("Role").is_some()
                || s.get("body")
                    .and_then(|b| b.get("STs").or_else(|| b.get("Role")))
                    .is_some()
        });

        // email role
        if let Some(role) = stats_block
            .and_then(|s| {
                s.get("Role")
                    .or_else(|| s.get("body").and_then(|b| b.get("Role")))
            })
            .and_then(Value::as_str)
        {
            map_insert_str_if_absent(meta, "email_role", Some(role));
        }

        // kvk season flag
        let is_kvk = stats_block
            .and_then(|s| {
                s.get("isConquerSeason")
                    .or_else(|| s.get("body").and_then(|b| b.get("isConquerSeason")))
                    .and_then(Value::as_bool)
            })
            .unwrap_or(false);
        meta.entry("is_kvk").or_insert(Value::from(is_kvk as i32));

        Ok(())
    }
}
