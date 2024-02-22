use sea_orm::{prelude::*, JoinType, QuerySelect};
use teloxide::prelude::*;

use crate::entity::{profiles, secondary_owners, statuses};

pub fn select_active_profiles() -> Select<profiles::Entity> {
    profiles::Entity::find()
        .join(
            JoinType::InnerJoin,
            profiles::Relation::MonitoringStatuses.def(),
        )
        // Is enabled
        .filter(statuses::Column::Enabled.eq(true))
        // Is there at least 1 emergency contact
        .join_rev(
            JoinType::InnerJoin,
            secondary_owners::Entity::belongs_to(profiles::Entity)
                .from(secondary_owners::Column::PrimaryOwnerChatId)
                .to(profiles::Column::ChatId)
                .into(),
        )
        .group_by(profiles::Column::Id)
        .having(Expr::cust("COUNT(secondary_owners.id) > 0"))
}

pub fn select_profile(chat_id: ChatId) -> Select<profiles::Entity> {
    profiles::Entity::find().filter(profiles::Column::ChatId.eq(chat_id.0))
}

pub fn select_emergency_contacts(chat_id: ChatId) -> Select<secondary_owners::Entity> {
    secondary_owners::Entity::find()
        .filter(secondary_owners::Column::PrimaryOwnerChatId.eq(chat_id.0))
}
