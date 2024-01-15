use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SecondaryOwners::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SecondaryOwners::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SecondaryOwners::PrimaryOwnerChatId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SecondaryOwners::SecondaryOwnerChatId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SecondaryOwners::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SecondaryOwners {
    Table,
    Id,
    PrimaryOwnerChatId,
    SecondaryOwnerChatId,
}
