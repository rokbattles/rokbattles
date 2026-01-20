use anyhow::Result;
use std::sync::Arc;
use twilight_http::Client as HttpClient;
use twilight_model::{
    application::command::{Command as DiscordCommand, CommandType},
    application::interaction::Interaction,
    id,
};

pub struct CommandContext {
    pub interaction: Interaction,
    pub http: Arc<HttpClient>,
}

impl CommandContext {
    // TODO helper functions like reply, defer, etc.
}

pub struct CommandMetadataBuilder {
    command: DiscordCommand,
}

#[allow(deprecated)]
impl CommandMetadataBuilder {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: DiscordCommand {
                application_id: None,
                default_member_permissions: None,
                // deprecated in twilight
                dm_permission: None,
                description: description.into(),
                description_localizations: None,
                guild_id: None,
                id: None,
                kind: CommandType::ChatInput,
                name: name.into(),
                name_localizations: None,
                nsfw: None,
                options: Vec::new(),
                version: id::Id::new(1),
                contexts: None,
                integration_types: None,
            },
        }
    }

    pub fn build(self) -> DiscordCommand {
        self.command
    }
}

pub trait CommandHandler: Send + 'static {
    fn handle(&self, ctx: CommandContext) -> Box<dyn Future<Output = Result<()>> + Send + 'static>;
}

impl<F, Fut> CommandHandler for F
where
    F: Fn(CommandContext) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    fn handle(&self, ctx: CommandContext) -> Box<dyn Future<Output = Result<()>> + Send + 'static> {
        Box::new(self(ctx))
    }
}

pub struct Command {
    pub name: String,
    pub metadata: DiscordCommand,
    pub handler: Box<dyn CommandHandler>,
}

impl Command {
    pub fn new<H>(name: impl Into<String>, metadata: DiscordCommand, handler: H) -> Self
    where
        H: CommandHandler + 'static,
    {
        Self {
            name: name.into(),
            metadata,
            handler: Box::new(handler),
        }
    }

    pub fn to_discord_command(&self) -> DiscordCommand {
        self.metadata.clone()
    }
}
