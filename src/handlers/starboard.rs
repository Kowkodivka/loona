use poise::serenity_prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, sea_query::OnConflict};
use tracing::warn;

use crate::{
    Data,
    entities::{prelude::*, starboard_posts},
    utils::starboard::get_or_default,
};

const STARBOARD_EMBED_COLOUR: Colour = Colour::new(0xE8B86D);

// TODO: опция игнорировать nsfw

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
        FullEvent::MessageDelete {
            deleted_message_id, ..
        } => {
            if let Err(err) = on_source_message_gone(ctx, &data.db_conn, *deleted_message_id).await
            {
                warn!("Failed to handle source message delete: {err:?}");
            }
        }
        FullEvent::MessageUpdate { event, .. } => {
            if let Err(err) = on_source_message_edited(ctx, &data.db_conn, event).await {
                warn!("Failed to handle source message update: {err:?}");
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

    if message.author.bot {
        return Ok(());
    }

    let count = message
        .reaction_users(&ctx.http, target_emoji.clone(), None, None)
        .await?
        .iter()
        .filter(|user| user.id != message.author.id)
        .count() as i64;

    let source_message_id = message.id.get() as i64;
    let starboard_channel = ChannelId::new(channel_id as u64);

    let existing = StarboardPosts::find_by_id(source_message_id)
        .one(db_conn)
        .await?;

    if count < settings.threshold {
        if let Some(post) = existing {
            delete_starboard_message(ctx, &starboard_channel, &post, db_conn).await?;
        }

        return Ok(());
    }

    let header = build_header_content(&message, count, &settings.emoji).await?;
    let embed = build_embed(&message);

    match existing {
        Some(post) => {
            starboard_channel
                .edit_message(
                    &ctx.http,
                    MessageId::new(post.starboard_message_id as u64),
                    EditMessage::new().content(header).embed(embed),
                )
                .await?;

            upsert_post(db_conn, &post, guild_id, &message, count).await?;
        }
        None => {
            let sent = starboard_channel
                .send_message(&ctx.http, CreateMessage::new().content(header).embed(embed))
                .await?;

            insert_new_post(db_conn, guild_id, &message, sent.id, count).await?;
        }
    }

    Ok(())
}

async fn on_source_message_gone(
    ctx: &Context,
    db_conn: &DatabaseConnection,
    deleted_message_id: MessageId,
) -> anyhow::Result<()> {
    let source_message_id = deleted_message_id.get() as i64;

    let Some(post) = StarboardPosts::find_by_id(source_message_id)
        .one(db_conn)
        .await?
    else {
        return Ok(());
    };

    let settings = get_or_default(db_conn, post.guild_id).await?;

    let Some(channel_id) = settings.channel_id else {
        return Ok(());
    };

    let starboard_channel = ChannelId::new(channel_id as u64);
    delete_starboard_message(ctx, &starboard_channel, &post, db_conn).await
}

async fn on_source_message_edited(
    ctx: &Context,
    db_conn: &DatabaseConnection,
    event: &MessageUpdateEvent,
) -> anyhow::Result<()> {
    let source_message_id = event.id.get() as i64;

    let Some(post) = StarboardPosts::find_by_id(source_message_id)
        .one(db_conn)
        .await?
    else {
        return Ok(());
    };

    let settings = get_or_default(db_conn, post.guild_id).await?;

    let Some(channel_id) = settings.channel_id else {
        return Ok(());
    };

    let message = event.channel_id.message(&ctx.http, event.id).await?;
    let header = build_header_content(&message, post.reaction_count, &settings.emoji).await?;
    let embed = build_embed(&message);

    ChannelId::new(channel_id as u64)
        .edit_message(
            &ctx.http,
            MessageId::new(post.starboard_message_id as u64),
            EditMessage::new().content(header).embed(embed),
        )
        .await?;

    Ok(())
}

async fn build_header_content(
    message: &Message,
    count: i64,
    emoji: &str,
) -> anyhow::Result<String> {
    let message_link = message.link();
    Ok(format!("{emoji} {count} • {message_link}"))
}

fn build_embed(message: &Message) -> CreateEmbed {
    let author_name = message.author.name.clone();
    let author_icon = message.author.face();

    let mut embed = CreateEmbed::new()
        .colour(STARBOARD_EMBED_COLOUR)
        .author(CreateEmbedAuthor::new(author_name).icon_url(author_icon))
        .description(message.content.clone())
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

    embed
}

async fn delete_starboard_message(
    ctx: &Context,
    starboard_channel: &ChannelId,
    post: &starboard_posts::Model,
    db_conn: &DatabaseConnection,
) -> anyhow::Result<()> {
    let _ = starboard_channel
        .delete_message(&ctx.http, MessageId::new(post.starboard_message_id as u64))
        .await;

    StarboardPosts::delete_by_id(post.source_message_id)
        .exec(db_conn)
        .await?;

    Ok(())
}

async fn insert_new_post(
    db_conn: &DatabaseConnection,
    guild_id: GuildId,
    message: &Message,
    starboard_message_id: MessageId,
    count: i64,
) -> anyhow::Result<()> {
    let active = starboard_posts::ActiveModel {
        source_message_id: sea_orm::ActiveValue::Set(message.id.get() as i64),
        guild_id: sea_orm::ActiveValue::Set(guild_id.get() as i64),
        source_channel_id: sea_orm::ActiveValue::Set(message.channel_id.get() as i64),
        starboard_message_id: sea_orm::ActiveValue::Set(starboard_message_id.get() as i64),
        reaction_count: sea_orm::ActiveValue::Set(count),
    };

    StarboardPosts::insert(active)
        .on_conflict(
            OnConflict::column(starboard_posts::Column::SourceMessageId)
                .update_column(starboard_posts::Column::ReactionCount)
                .to_owned(),
        )
        .exec(db_conn)
        .await?;

    Ok(())
}

async fn upsert_post(
    db_conn: &DatabaseConnection,
    existing: &starboard_posts::Model,
    guild_id: GuildId,
    message: &Message,
    count: i64,
) -> anyhow::Result<()> {
    insert_new_post(
        db_conn,
        guild_id,
        message,
        MessageId::new(existing.starboard_message_id as u64),
        count,
    )
    .await
}
