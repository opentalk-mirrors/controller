// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::{schema::event_shared_folders, tables::event_shared_folders::EventSharedFolder};

#[derive(Insertable)]
#[diesel(table_name = event_shared_folders)]
pub struct NewEventSharedFolder {
    pub event_id: EventId,
    pub path: String,
    pub write_share_id: String,
    pub write_url: String,
    pub write_password: String,
    pub read_share_id: String,
    pub read_url: String,
    pub read_password: String,
}

impl From<inventory::NewEventSharedFolder> for NewEventSharedFolder {
    fn from(
        inventory::NewEventSharedFolder {
            event_id,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }: inventory::NewEventSharedFolder,
    ) -> Self {
        Self {
            event_id,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }
    }
}

impl NewEventSharedFolder {
    /// Tries to insert the EventSharedFolder into the database.
    ///
    /// When yielding a unique constraint violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<EventSharedFolder>> {
        let query = self.insert_into(event_shared_folders::table);

        let result = query.get_result(conn).await;

        match result {
            Ok(event_shared_folders) => Ok(Some(event_shared_folders)),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..,
            )) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
