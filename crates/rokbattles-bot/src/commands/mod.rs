mod help;
mod mailcache;

use crate::framework::CommandRegistry;

#[must_use]
pub fn build_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(help::help_command());
    registry.register(mailcache::mailcache_command());

    registry
}

#[cfg(test)]
mod tests {
    use super::build_registry;

    #[test]
    fn registry_contains_required_commands() {
        let registry = build_registry();
        assert!(registry.contains("help"));
        assert!(registry.contains("mailcache"));
        assert_eq!(registry.len(), 2);
    }
}
