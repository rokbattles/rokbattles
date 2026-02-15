#![forbid(unsafe_code)]

mod commands;
mod framework;

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{error, info, warn};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client as HttpClient;
use twilight_model::id::{Id, marker::ApplicationMarker};

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
struct Config {
    discord_token: String,
    discord_application_id: Id<ApplicationMarker>,
}

impl Config {
    fn from_env() -> Result<Self> {
        let discord_token = std::env::var("DISCORD_TOKEN")
            .context("missing required environment variable DISCORD_TOKEN")?;

        let application_id = std::env::var("DISCORD_APPLICATION_ID")
            .context("missing required environment variable DISCORD_APPLICATION_ID")?;
        let discord_application_id =
            parse_id::<ApplicationMarker>("DISCORD_APPLICATION_ID", application_id.as_str())?;

        Ok(Self {
            discord_token,
            discord_application_id,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

async fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    install_rustls_crypto_provider()?;
    init_tracing();

    let config = Config::from_env()?;
    let http = Arc::new(HttpClient::new(config.discord_token.clone()));
    let registry = Arc::new(commands::build_registry());

    registry
        .deploy_commands(http.clone(), config.discord_application_id)
        .await?;

    info!("deployed global slash commands");

    run_gateway(config, http, registry).await
}

async fn run_gateway(
    config: Config,
    http: Arc<HttpClient>,
    registry: Arc<framework::CommandRegistry>,
) -> Result<()> {
    let mut shard = Shard::new(ShardId::ONE, config.discord_token, Intents::GUILDS);

    while let Some(next_event) = shard.next_event(EventTypeFlags::all()).await {
        let event = match next_event {
            Ok(event) => event,
            Err(source) => {
                warn!(?source, "gateway receive error");
                continue;
            }
        };

        match event {
            Event::Ready(ready) => {
                info!(
                    user_id = %ready.user.id,
                    username = %ready.user.name,
                    "connected to Discord gateway"
                );
            }
            Event::InteractionCreate(interaction) => {
                if let Err(source) = registry
                    .handle_interaction(interaction.0.clone(), http.clone())
                    .await
                {
                    error!(?source, "failed to handle interaction");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "rokbattles_bot=info,twilight_gateway=warn".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn parse_id<M>(name: &str, value: &str) -> Result<Id<M>> {
    let parsed = value
        .parse::<u64>()
        .with_context(|| format!("invalid {name} value: {value}"))?;

    Ok(Id::new(parsed))
}

fn install_rustls_crypto_provider() -> Result<()> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| anyhow::anyhow!("failed to install rustls ring crypto provider"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_id;
    use twilight_model::id::marker::ApplicationMarker;

    #[test]
    fn parse_id_accepts_u64() {
        let id = parse_id::<ApplicationMarker>("DISCORD_APPLICATION_ID", "123").unwrap();
        assert_eq!(id.get(), 123);
    }

    #[test]
    fn parse_id_rejects_non_numeric() {
        let error = parse_id::<ApplicationMarker>("DISCORD_APPLICATION_ID", "abc").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid DISCORD_APPLICATION_ID value: abc")
        );
    }
}
