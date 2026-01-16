use std::{env, sync::Arc};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{EventTypeFlags, Shard, StreamExt};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::{Intents, ShardId};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,twilight=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    let http = Arc::new(HttpClient::new(token.clone()));
    let _application_id = http.current_user_application().await?.model().await?.id;

    // TODO commands

    let mut shard = Shard::new(ShardId::ONE, token.clone(), Intents::empty());

    info!("bot is now running");

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(_event) = item else {
            continue;
        };

        // TODO interaction
    }

    Ok(())
}
