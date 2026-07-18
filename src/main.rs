mod commands;
mod config;
mod entities;
mod handlers;
mod translation;
mod utils;

use fluent_templates::static_loader;
use migration::{Migrator, MigratorTrait};
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions, serenity_prelude::*};
use sea_orm::{Database, DatabaseConnection};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

#[allow(dead_code)]
struct Data {
    config: Config,
    db_conn: DatabaseConnection,
}

const CONFIG_PATH: &'static str = "config.toml";

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en-US"
    };
}

// TODO: добавить help команду
// TODO: добавить настройки языка для отдельного пользователя

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::from("info")))
        .init();

    let config = Config::load(CONFIG_PATH).await?;

    let db_conn = Database::connect(&config.database.url).await?;
    Migrator::up(&db_conn, None).await?;

    let mut commands = vec![commands::starboard::starboard()];
    translation::apply_translations(&LOCALES, &mut commands);

    let framework = Framework::builder()
        .options(FrameworkOptions {
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("l!".to_owned()),
                ..Default::default()
            },
            commands,
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
