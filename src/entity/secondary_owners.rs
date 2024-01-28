use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "secondary_owners")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub primary_owner_chat_id: i64,
    pub secondary_owner_chat_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
