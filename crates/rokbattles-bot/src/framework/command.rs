use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::{
        command::{Command as DiscordCommand, CommandOption, CommandType},
        interaction::{Interaction, InteractionData, application_command::CommandData},
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id,
};

/// Shared command execution context.
#[derive(Clone)]
pub struct CommandContext {
    pub interaction: Interaction,
    pub http: Arc<HttpClient>,
}

impl CommandContext {
    /// Returns application command payload if this interaction is a slash command.
    pub fn command_data(&self) -> Option<&CommandData> {
        match self.interaction.data.as_ref() {
            Some(InteractionData::ApplicationCommand(data)) => Some(data),
            _ => None,
        }
    }

    /// Respond to the interaction with message content.
    pub async fn reply(&self, content: impl Into<String>) -> Result<()> {
        let data = InteractionResponseData {
            content: Some(content.into()),
            ..Default::default()
        };

        self.http
            .interaction(self.interaction.application_id)
            .create_response(
                self.interaction.id,
                &self.interaction.token,
                &InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(data),
                },
            )
            .await
            .context("failed to send interaction response")?;

        Ok(())
    }
}

/// Heap-allocated async command future.
pub type BoxCommandFuture = Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

/// Trait implemented by command handlers.
pub trait CommandHandler: Send + Sync + 'static {
    fn handle(&self, ctx: CommandContext) -> BoxCommandFuture;
}

impl<F, Fut> CommandHandler for F
where
    F: Fn(CommandContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn handle(&self, ctx: CommandContext) -> BoxCommandFuture {
        Box::pin(self(ctx))
    }
}

/// Helper for building Discord command metadata.
pub struct CommandMetadataBuilder {
    command: DiscordCommand,
}

#[allow(deprecated)]
impl CommandMetadataBuilder {
    /// Create a chat-input command metadata object.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: DiscordCommand {
                application_id: None,
                contexts: None,
                default_member_permissions: None,
                dm_permission: None,
                description: description.into(),
                description_localizations: None,
                guild_id: None,
                id: None,
                integration_types: None,
                kind: CommandType::ChatInput,
                name: name.into(),
                name_localizations: None,
                nsfw: None,
                options: Vec::new(),
                version: id::Id::new(1),
            },
        }
    }

    /// Set command options.
    pub fn with_options(mut self, options: Vec<CommandOption>) -> Self {
        self.command.options = options;
        self
    }

    /// Build command metadata.
    pub fn build(self) -> DiscordCommand {
        self.command
    }
}

/// Command metadata and execution handler.
pub struct Command {
    pub name: String,
    pub metadata: DiscordCommand,
    pub handler: Box<dyn CommandHandler>,
}

impl Command {
    /// Construct a new command.
    pub fn new<H>(name: impl Into<String>, metadata: DiscordCommand, handler: H) -> Self
    where
        H: CommandHandler,
    {
        Self {
            name: name.into(),
            metadata,
            handler: Box::new(handler),
        }
    }

    /// Clone metadata for Discord command deployment.
    pub fn to_discord_command(&self) -> DiscordCommand {
        self.metadata.clone()
    }
}
