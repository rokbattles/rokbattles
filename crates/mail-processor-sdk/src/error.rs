use std::{error::Error, fmt};

/// Boxed error type used by the resolver pipeline.
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// Error returned when a resolver step fails.
#[derive(Debug)]
pub struct ResolverError {
    step: String,
    source: BoxError,
}

impl ResolverError {
    /// Creates a new resolver error for the named step.
    pub fn new(step: impl Into<String>, source: BoxError) -> Self {
        Self {
            step: step.into(),
            source,
        }
    }

    /// Returns the name of the resolver step that failed.
    pub fn step(&self) -> &str {
        &self.step
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "resolver step '{}' failed: {}", self.step, self.source)
    }
}

impl Error for ResolverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
