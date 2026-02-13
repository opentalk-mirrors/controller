// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::Insertable;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::{events::EventId, users::UserId};

use crate::{schema::event_favorites, tables::event_favorites::EventFavorite};

#[derive(Insertable)]
#[diesel(table_name = event_favorites)]
pub struct NewEventFavorite {
    pub user_id: UserId,
    pub event_id: EventId,
}

impl NewEventFavorite {
    /// Tries to insert the NewEventFavorite into the database
    ///
    /// When yielding a unique key violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<EventFavorite>> {
        let query = self.insert_into(event_favorites::table);

        let result = query.get_result(conn).await;

        match result {
            Ok(event_favorite) => Ok(Some(event_favorite)),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..,
            )) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
