use serde_json::Value;

/// Read-only context shared with resolver steps.
pub struct MailContext<'a> {
    /// Decoded mail sections in the order they appear in the raw payload.
    pub sections: &'a [Value],
}

impl<'a> MailContext<'a> {
    /// Creates a new context for the provided mail sections.
    pub fn new(sections: &'a [Value]) -> Self {
        Self { sections }
    }
}
