use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MonitoringStatuses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MonitoringStatuses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(MonitoringStatuses::ChatId)
                            .unique_key()
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MonitoringStatuses::Enabled)
                            .boolean()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MonitoringStatuses::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MonitoringStatuses {
    Table,
    Id,
    ChatId,
    Enabled,
}
