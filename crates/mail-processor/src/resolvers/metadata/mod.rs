use crate::resolvers::{Resolver, ResolverChain, ResolverContext};
use serde_json::Value;

mod email;
mod players;
mod position;
mod role;
mod time;

pub struct MetadataResolver {
    chain: ResolverChain,
}

impl Default for MetadataResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataResolver {
    pub fn new() -> Self {
        let chain = ResolverChain::new()
            .with(email::EmailBasicsResolver::new())
            .with(role::RoleSeasonResolver::new())
            .with(time::TimeResolver::new())
            .with(position::PositionResolver::new())
            .with(players::PlayersResolver::new());
        Self { chain }
    }
}

impl Resolver for MetadataResolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> anyhow::Result<()> {
        self.chain.apply(ctx, mail)
    }
}
