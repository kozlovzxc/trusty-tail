use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Invites::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Invites::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Invites::ChatId)
                            .unique_key()
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Invites::Invite).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invites::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Invites {
    Table,
    Id,
    ChatId,
    Invite,
}
