use serde_json::Value;

/// Read-only context shared with resolver steps.
pub struct MailContext<'a> {
    /// Decoded mail sections in the order they appear in the raw payload.
    pub sections: &'a [Value],
    /// Sections that belong to the current battle group.
    pub group: &'a [Value],
    /// Attack identifier for the current battle group.
    pub attack_id: &'a str,
}

impl<'a> MailContext<'a> {
    /// Creates a new context for the provided mail sections.
    pub fn new(sections: &'a [Value], group: &'a [Value], attack_id: &'a str) -> Self {
        Self {
            sections,
            group,
            attack_id,
        }
    }
}
