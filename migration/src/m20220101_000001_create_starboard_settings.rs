use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("starboard_settings")
                    .if_not_exists()
                    .col(big_unsigned("guild_id").primary_key())
                    .col(boolean("enabled").default(false))
                    .col(integer("threshold").default(3))
                    .col(string("emoji").default("⭐"))
                    .col(big_unsigned_null("channel_id"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("starboard_settings").to_owned())
            .await
    }
}
