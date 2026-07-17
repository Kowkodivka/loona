use poise::serenity_prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, ModelTrait};
use tracing::warn;

use crate::{
    Data,
    entities::{prelude::*, starboard_posts},
    utils::starboard::get_or_default,
};

// TODO: сделать стиль как у Carl'а
// TODO: игнорировать ботов

pub async fn handle_event(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, anyhow::Error>,
    data: &Data,
) -> anyhow::Result<()> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => {
            if let Err(err) = on_reaction_change(ctx, &data.db_conn, add_reaction).await {
                warn!("Failed to handle reaction add: {err:?}");
            }
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            if let Err(err) = on_reaction_change(ctx, &data.db_conn, removed_reaction).await {
                warn!("Failed to handle reaction remove: {err:?}");
            }
        }

        _ => {}
    }

    Ok(())
}

async fn on_reaction_change(
    ctx: &Context,
    db_conn: &DatabaseConnection,
    reaction: &Reaction,
) -> anyhow::Result<()> {
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    let settings = get_or_default(db_conn, guild_id.get() as i64).await?;

    if !settings.enabled {
        return Ok(());
    }

    let Some(channel_id) = settings.channel_id else {
        return Ok(());
    };

    let Ok(target_emoji) = settings.emoji.parse::<ReactionType>() else {
        warn!(
            "Failed to parse saved emoji '{}' for guild {guild_id}",
            settings.emoji
        );
        return Ok(());
    };

    if reaction.emoji != target_emoji {
        return Ok(());
    }

    let message = reaction.message(&ctx.http).await?;

    let count = message
        .reaction_users(&ctx.http, target_emoji.clone(), None, None)
        .await?
        .len() as i64;

    let source_message_id = message.id.get() as i64;

    let existing = StarboardPosts::find_by_id(source_message_id)
        .one(db_conn)
        .await?;

    if count < settings.threshold {
        if let Some(post) = existing {
            let starboard_channel = ChannelId::new(channel_id as u64);
            let _ = starboard_channel
                .delete_message(&ctx.http, MessageId::new(post.starboard_message_id as u64))
                .await;

            post.delete(db_conn).await?;
        }

        return Ok(());
    }

    let starboard_channel = ChannelId::new(channel_id as u64);

    let author_name = message.author.name.clone();
    let author_icon = message.author.face();

    let mut embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::new(author_name).icon_url(author_icon))
        .description(message.content.clone())
        .footer(CreateEmbedFooter::new(format!(
            "{} {} • #{}",
            count,
            settings.emoji,
            message.channel_id.name(&ctx.http).await.unwrap_or_default()
        )))
        .timestamp(message.timestamp);

    if let Some(attachment) = message.attachments.first() {
        if attachment
            .content_type
            .as_deref()
            .is_some_and(|ct| ct.starts_with("image/"))
        {
            embed = embed.image(&attachment.url);
        }
    }

    match existing {
        Some(post) => {
            starboard_channel
                .edit_message(
                    &ctx.http,
                    MessageId::new(post.starboard_message_id as u64),
                    EditMessage::new().embed(embed),
                )
                .await?;

            let mut active: starboard_posts::ActiveModel = post.into();
            active.reaction_count = Set(count);
            active.update(db_conn).await?;
        }
        None => {
            let sent = starboard_channel
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await?;

            let active = starboard_posts::ActiveModel {
                source_message_id: Set(source_message_id),
                guild_id: Set(guild_id.get() as i64),
                source_channel_id: Set(message.channel_id.get() as i64),
                starboard_message_id: Set(sent.id.get() as i64),
                reaction_count: Set(count),
            };

            active.insert(db_conn).await?;
        }
    }

    Ok(())
}
