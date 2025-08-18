use anyhow::Result;
use serde_json::Value;

pub trait Resolver {
    fn resolve(&self, mail: &mut Value) -> Result<()>;
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

    pub fn apply(&self, v: &mut Value) -> Result<()> {
        for r in &self.steps {
            r.resolve(v)?;
        }
        Ok(())
    }
}
