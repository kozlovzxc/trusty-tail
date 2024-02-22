use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "monitoring_statuses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub chat_id: i64,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::profiles::Entity",
        from = "Column::ChatId",
        to = "super::profiles::Column::ChatId"
    )]
    Profiles,
}

impl Related<super::profiles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Profiles.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
