use poise::serenity_prelude::{ChannelId, ReactionType};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};

use crate::{Context, entities::starboard_settings, utils::starboard::get_or_default};

// TODO: добавить локализованное описание

#[poise::command(
    slash_command,
    subcommands("enable", "disable", "threshold", "emoji", "channel"),
    default_member_permissions = "MANAGE_GUILD",
    guild_only
)]
pub async fn starboard(_: Context<'_>) -> anyhow::Result<()> {
    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn enable(ctx: Context<'_>) -> anyhow::Result<()> {
    set_enabled(ctx, true).await?;
    ctx.say("Starboard enabled").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn disable(ctx: Context<'_>) -> anyhow::Result<()> {
    set_enabled(ctx, false).await?;
    ctx.say("Starboard disabled").await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn threshold(ctx: Context<'_>, value: i64) -> anyhow::Result<()> {
    if value < 1 {
        ctx.say("Threshold must be >= 1").await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let settings = get_or_default(&ctx.data().db_conn, guild_id).await?;

    let mut active: starboard_settings::ActiveModel = settings.into();
    active.threshold = Set(value);
    active.update(&ctx.data().db_conn).await?;

    ctx.say(format!("Threshold set: {value}")).await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn emoji(ctx: Context<'_>, value: String) -> anyhow::Result<()> {
    if value.parse::<ReactionType>().is_err() {
        ctx.say(
            "Emoji could not be recognized. Use a standard Unicode emoji or a custom server emoji.",
        )
        .await?;
        return Ok(());
    }

    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let settings = get_or_default(&ctx.data().db_conn, guild_id).await?;

    let mut active: starboard_settings::ActiveModel = settings.into();
    active.emoji = Set(value.clone());
    active.update(&ctx.data().db_conn).await?;

    ctx.say(format!("Emoji set: {value}")).await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
async fn channel(ctx: Context<'_>, value: ChannelId) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let settings = get_or_default(&ctx.data().db_conn, guild_id).await?;

    let mut active: starboard_settings::ActiveModel = settings.into();
    active.channel_id = Set(Some(value.get() as i64));
    active.update(&ctx.data().db_conn).await?;

    ctx.say(format!("Channel set: <#{value}>")).await?;

    Ok(())
}

async fn set_enabled(ctx: Context<'_>, enabled: bool) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let settings = get_or_default(&ctx.data().db_conn, guild_id).await?;

    let mut active: starboard_settings::ActiveModel = settings.into();
    active.enabled = Set(enabled);
    active.update(&ctx.data().db_conn).await?;

    Ok(())
}
