// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::events::shared_folders::{self as db};
use opentalk_inventory::{EventSharedFolder, EventSharedFolderInventory, NewEventSharedFolder};
use opentalk_types_common::{events::EventId, rooms::RoomId};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl EventSharedFolderInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn try_create_event_shared_folder(
        &mut self,
        new_shared_folder: NewEventSharedFolder,
    ) -> Result<Option<EventSharedFolder>> {
        Ok(db::NewEventSharedFolder::from(new_shared_folder)
            .try_insert(&mut self.inner)
            .await
            .context(DatabaseSnafu)?
            .map(Into::into))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_shared_folder(
        &mut self,
        event_id: EventId,
    ) -> Result<Option<EventSharedFolder>> {
        Ok(
            db::EventSharedFolder::get_for_event(&mut self.inner, event_id)
                .await
                .context(DatabaseSnafu)?
                .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_shared_folders_for_room(
        &mut self,
        room_id: RoomId,
    ) -> Result<Vec<EventSharedFolder>> {
        Ok(
            db::EventSharedFolder::get_all_for_room(&mut self.inner, room_id)
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_shared_folder_by_event_id(&mut self, event_id: EventId) -> Result<()> {
        Ok(
            db::EventSharedFolder::delete_by_event_id(&mut self.inner, event_id)
                .await
                .context(DatabaseSnafu)?,
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_shared_folders_by_event_ids(&mut self, event_ids: &[EventId]) -> Result<()> {
        Ok(
            db::EventSharedFolder::delete_by_event_ids(&mut self.inner, event_ids)
                .await
                .context(DatabaseSnafu)?,
        )
    }
}
