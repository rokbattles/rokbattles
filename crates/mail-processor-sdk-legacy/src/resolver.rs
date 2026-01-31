use std::error::Error;

/// A single resolver step in a processing pipeline.
///
/// Implementations should inspect the context and mutate the output accordingly.
pub trait Resolver<C, O> {
    /// Error type returned by this resolver.
    type Error: Error + Send + Sync + 'static;

    /// Applies this resolver step to the output.
    fn resolve(&self, ctx: &C, output: &mut O) -> Result<(), Self::Error>;

    /// Returns the step name used for error reporting.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
