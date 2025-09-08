use crate::helpers::{
    find_self_content_root, find_self_snapshot_section, get_or_insert_object,
    map_insert_i64_if_absent, map_insert_str_if_absent,
};
use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct EmailBasicsResolver;

impl Default for EmailBasicsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailBasicsResolver {
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
}

impl Resolver for EmailBasicsResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        let meta = match get_or_insert_object(mail, "metadata") {
            Value::Object(meta) => meta,
            _ => unreachable!("metadata must be an object"),
        };

        // attack id
        meta.entry("attack_id")
            .or_insert_with(|| Value::String(ctx.attack_id.to_string()));

        // email basics
        if let Some(g0) = ctx.sections.first() {
            map_insert_str_if_absent(meta, "email_id", g0.get("id").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_type", g0.get("type").and_then(Value::as_str));
            map_insert_str_if_absent(meta, "email_box", g0.get("box").and_then(Value::as_str));
            map_insert_i64_if_absent(
                meta,
                "email_time",
                Self::parse_i128(g0.get("time")).and_then(|n| i64::try_from(n).ok()),
            );
        }

        // email receiver
        let self_snap = find_self_snapshot_section(ctx.sections).unwrap_or(&Value::Null);
        let self_body = find_self_content_root(ctx.sections).unwrap_or(&Value::Null);
        if let Some(pid) = self_snap
            .get("PId")
            .and_then(Value::as_i64)
            .or_else(|| self_body.pointer("/SelfChar/PId").and_then(Value::as_i64))
            && pid != 0
        {
            let s = pid.to_string();
            map_insert_str_if_absent(meta, "email_receiver", Some(&s));
        }

        Ok(())
    }
}
