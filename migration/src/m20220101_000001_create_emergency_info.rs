use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmergencyInfo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmergencyInfo::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EmergencyInfo::ChatId)
                            .unique_key()
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EmergencyInfo::Text).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmergencyInfo::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum EmergencyInfo {
    Table,
    Id,
    ChatId,
    Text,
}
