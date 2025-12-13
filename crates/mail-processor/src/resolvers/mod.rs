pub mod battle;
pub mod metadata;
pub mod overview;
pub mod participant_enemy;
pub mod participant_self;

use anyhow::Result;
use serde_json::Value;

pub struct ResolverContext<'a> {
    pub sections: &'a [Value],
    pub group: &'a [Value],
    pub attack_id: &'a str,
}

pub trait Resolver {
    fn resolve(&self, ctx: &ResolverContext<'_>, mail: &mut Value) -> Result<()>;
}

pub struct ResolverChain {
    steps: Vec<Box<dyn Resolver + Send + Sync>>,
}

impl Default for ResolverChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolverChain {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn with(mut self, r: impl Resolver + Send + Sync + 'static) -> Self {
        self.steps.push(Box::new(r));
        self
    }

    pub fn apply(&self, ctx: &ResolverContext<'_>, v: &mut Value) -> Result<()> {
        for r in &self.steps {
            r.resolve(ctx, v)?;
        }
        Ok(())
    }
}
