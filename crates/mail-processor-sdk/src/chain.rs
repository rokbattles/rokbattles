use crate::error::{BoxError, ResolverError};
use crate::resolver::Resolver;

/// Ordered collection of resolver steps.
pub struct ResolverChain<C, O> {
    steps: Vec<ResolverStep<C, O>>,
}

impl<C, O> Default for ResolverChain<C, O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C, O> ResolverChain<C, O> {
    /// Creates an empty resolver chain.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Appends a resolver step, using its default name.
    pub fn with<R>(mut self, resolver: R) -> Self
    where
        R: Resolver<C, O> + Send + Sync + 'static,
    {
        self.push(resolver);
        self
    }

    /// Appends a resolver step with an explicit name.
    pub fn with_named<R>(mut self, name: impl Into<String>, resolver: R) -> Self
    where
        R: Resolver<C, O> + Send + Sync + 'static,
    {
        self.push_named(name, resolver);
        self
    }

    /// Pushes a resolver step into the chain, using its default name.
    pub fn push<R>(&mut self, resolver: R) -> &mut Self
    where
        R: Resolver<C, O> + Send + Sync + 'static,
    {
        let name = resolver.name().to_string();
        self.steps.push(ResolverStep::new(name, resolver));
        self
    }

    /// Pushes a resolver step into the chain with an explicit name.
    pub fn push_named<R>(&mut self, name: impl Into<String>, resolver: R) -> &mut Self
    where
        R: Resolver<C, O> + Send + Sync + 'static,
    {
        self.steps.push(ResolverStep::new(name.into(), resolver));
        self
    }

    /// Applies each resolver in order, stopping at the first error.
    pub fn apply(&self, ctx: &C, output: &mut O) -> Result<(), ResolverError> {
        for step in &self.steps {
            if let Err(source) = step.resolver.resolve_dyn(ctx, output) {
                // Wrap the source error with the step name for better diagnostics.
                return Err(ResolverError::new(step.name.clone(), source));
            }
        }
        Ok(())
    }
}

struct ResolverStep<C, O> {
    name: String,
    resolver: Box<dyn ResolverDyn<C, O>>,
}

impl<C, O> ResolverStep<C, O> {
    fn new<R>(name: String, resolver: R) -> Self
    where
        R: Resolver<C, O> + Send + Sync + 'static,
    {
        Self {
            name,
            resolver: Box::new(resolver),
        }
    }
}

trait ResolverDyn<C, O>: Send + Sync {
    fn resolve_dyn(&self, ctx: &C, output: &mut O) -> Result<(), BoxError>;
}

impl<C, O, R> ResolverDyn<C, O> for R
where
    R: Resolver<C, O> + Send + Sync,
{
    fn resolve_dyn(&self, ctx: &C, output: &mut O) -> Result<(), BoxError> {
        // Box errors to unify the chain's error handling.
        self.resolve(ctx, output)
            .map_err(|err| Box::new(err) as BoxError)
    }
}
