use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AliveEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AliveEvents::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AliveEvents::ChatId)
                            .unique_key()
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AliveEvents::Timestamp)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AliveEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AliveEvents {
    Table,
    Id,
    ChatId,
    Timestamp,
}
