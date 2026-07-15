use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::entities::{prelude::*, starboard_settings};

pub async fn get_or_default(
    db_conn: &DatabaseConnection,
    guild_id: i64,
) -> anyhow::Result<starboard_settings::Model> {
    if let Some(existing) = StarboardSettings::find_by_id(guild_id).one(db_conn).await? {
        return Ok(existing);
    }

    let active = starboard_settings::ActiveModel {
        guild_id: Set(guild_id),
        enabled: Set(false),
        threshold: Set(3),
        emoji: Set("⭐".to_string()),
        channel_id: Set(None),
    };

    Ok(active.insert(db_conn).await?)
}
