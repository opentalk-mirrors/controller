// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{BoolExpressionMethods, ExpressionMethods, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::{events::EventId, users::UserId};

use crate::{schema::event_favorites, tables::events::Event, users::User};

#[derive(Associations, Identifiable, Queryable)]
#[diesel(table_name = event_favorites)]
#[diesel(primary_key(user_id, event_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Event))]
pub struct EventFavorite {
    pub user_id: UserId,
    pub event_id: EventId,
}

impl EventFavorite {
    /// Deletes a EventFavorite entry by user_id and event_id
    ///
    /// Returns true if something was deleted
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(
        conn: &mut DbConnection,
        user_id: UserId,
        event_id: EventId,
    ) -> Result<bool> {
        let lines_changes = diesel::delete(event_favorites::table)
            .filter(
                event_favorites::user_id
                    .eq(user_id)
                    .and(event_favorites::event_id.eq(event_id)),
            )
            .execute(conn)
            .await?;

        Ok(lines_changes > 0)
    }
}
