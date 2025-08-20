use crate::resolvers::{Resolver, ResolverContext};
use serde_json::Value;

pub struct MetadataResolver {}

impl MetadataResolver {
    pub fn new() -> Self {
        Self {}
    }
}

impl Resolver for MetadataResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        Ok(())
    }
}
