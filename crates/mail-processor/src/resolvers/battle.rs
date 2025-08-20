use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct BattleResolver {}

impl BattleResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Resolver for BattleResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        Ok(())
    }
}
