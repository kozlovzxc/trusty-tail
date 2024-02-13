use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::monitoring_statuses;

pub async fn is_enabled(connection: &DatabaseConnection, chat_id: i64) -> bool {
    monitoring_statuses::Entity::find()
        .filter(monitoring_statuses::Column::ChatId.eq(chat_id))
        .one(connection)
        .await
        .ok()
        .flatten()
        .map_or(false, |x| x.enabled)
}
