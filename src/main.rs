mod commands;
mod config;
mod entities;
mod handlers;
mod utils;

use migration::{Migrator, MigratorTrait};
use poise::{Framework, FrameworkOptions, serenity_prelude::*};
use sea_orm::{Database, DatabaseConnection};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[allow(dead_code)]
struct Data {
    config: Config,
    db_conn: DatabaseConnection,
}

const CONFIG_PATH: &'static str = "data/config.toml";

// TODO: сделать систему перевода

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::from("info")))
        .init();

    let Some(config) = Config::load_or_init(CONFIG_PATH).await? else {
        warn!("Generated a new configuration file at {CONFIG_PATH}");
        warn!("Please review it and fill in the required fields before restarting");

        return Ok(());
    };

    let db_conn = Database::connect(&config.database.url).await?;

    Migrator::up(&db_conn, None).await?;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![commands::starboard::starboard()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(async move {
                    handlers::starboard::handle_event(ctx, event, framework, data).await
                })
            },
            ..Default::default()
        })
        .setup({
            let config = config.clone();
            move |ctx, ready, framework| {
                Box::pin(async move {
                    info!("Logged in as {}", ready.user.name);
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(Data { config, db_conn })
                })
            }
        })
        .build();

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = ClientBuilder::new(config.discord.token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
