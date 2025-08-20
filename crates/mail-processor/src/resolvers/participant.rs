use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct ParticipantResolver {}

impl ParticipantResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Resolver for ParticipantResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        Ok(())
    }
}
