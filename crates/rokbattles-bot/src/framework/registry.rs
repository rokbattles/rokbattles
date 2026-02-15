use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::interaction::{Interaction, InteractionData, InteractionType},
    id::{Id, marker::ApplicationMarker},
};

use crate::framework::{Command, CommandContext};

/// In-memory command registry and dispatcher.
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register or replace a command by its lowercase command name.
    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name.to_lowercase(), command);
    }

    /// Deploy command metadata to Discord.
    pub async fn deploy_commands(
        &self,
        http: Arc<HttpClient>,
        application_id: Id<ApplicationMarker>,
    ) -> Result<()> {
        let discord_commands = self
            .commands
            .values()
            .map(Command::to_discord_command)
            .collect::<Vec<_>>();

        http.interaction(application_id)
            .set_global_commands(&discord_commands)
            .await?;

        Ok(())
    }

    /// Dispatch a gateway interaction to the matched command handler.
    pub async fn handle_interaction(
        &self,
        interaction: Interaction,
        http: Arc<HttpClient>,
    ) -> Result<()> {
        if interaction.kind != InteractionType::ApplicationCommand {
            return Ok(());
        }

        let command_name = match interaction.data.as_ref() {
            Some(InteractionData::ApplicationCommand(data)) => data.name.to_lowercase(),
            _ => return Ok(()),
        };

        let Some(command) = self.commands.get(&command_name) else {
            return Ok(());
        };

        let context = CommandContext { interaction, http };
        command.handler.handle(context).await
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.commands.len()
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(&name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::framework::{Command, CommandContext, CommandMetadataBuilder, CommandRegistry};

    fn test_command(name: &str) -> Command {
        Command::new(
            name,
            CommandMetadataBuilder::new(name, "test command").build(),
            |_ctx: CommandContext| async move { Ok(()) },
        )
    }

    #[test]
    fn register_tracks_command_count() {
        let mut registry = CommandRegistry::new();
        registry.register(test_command("alpha"));
        registry.register(test_command("beta"));

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn register_is_case_insensitive() {
        let mut registry = CommandRegistry::new();
        registry.register(test_command("Help"));

        assert!(registry.contains("help"));
        assert!(registry.contains("HELP"));
    }

    #[test]
    fn register_replaces_existing_name() {
        let mut registry = CommandRegistry::new();
        registry.register(test_command("help"));
        registry.register(Command::new(
            "HELP",
            CommandMetadataBuilder::new("help", "overwritten").build(),
            |_ctx: CommandContext| async move { Result::<()>::Ok(()) },
        ));

        assert_eq!(registry.len(), 1);
    }
}
