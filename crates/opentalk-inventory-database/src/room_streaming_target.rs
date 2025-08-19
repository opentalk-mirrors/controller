// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::streaming_targets::{
    self as db, get_room_streaming_targets, insert_room_streaming_target,
    override_room_streaming_targets,
};
use opentalk_inventory::{
    RoomStreamingTargetInventory, RoomStreamingTargetRecord, UpdateRoomStreamingTarget,
};
use opentalk_types_common::{
    rooms::RoomId,
    streaming::{RoomStreamingTarget, StreamingTarget, StreamingTargetId},
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl RoomStreamingTargetInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn get_room_streaming_targets(
        &mut self,
        room_id: RoomId,
    ) -> Result<Vec<RoomStreamingTarget>> {
        Ok(get_room_streaming_targets(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_streaming_target_records(
        &mut self,
        room_id: RoomId,
    ) -> Result<Vec<RoomStreamingTargetRecord>> {
        Ok(
            db::RoomStreamingTargetRecord::get_all_for_room(&mut self.inner, room_id)
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_streaming_target_record(
        &mut self,
        room_id: RoomId,
        streaming_target_id: StreamingTargetId,
    ) -> Result<RoomStreamingTargetRecord> {
        Ok(
            db::RoomStreamingTargetRecord::get(&mut self.inner, streaming_target_id, room_id)
                .await
                .context(DatabaseSnafu)?
                .into(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn create_room_streaming_target(
        &mut self,
        room_id: RoomId,
        streaming_target: StreamingTarget,
    ) -> Result<RoomStreamingTarget> {
        Ok(
            insert_room_streaming_target(&mut self.inner, room_id, streaming_target)
                .await
                .context(DatabaseSnafu)?,
        )
    }

    async fn update_room_streaming_target(
        &mut self,
        room_id: RoomId,
        streaming_target_id: StreamingTargetId,
        streaming_target: UpdateRoomStreamingTarget,
    ) -> Result<RoomStreamingTargetRecord> {
        Ok(db::UpdateRoomStreamingTarget::from(streaming_target)
            .apply(&mut self.inner, room_id, streaming_target_id)
            .await
            .context(DatabaseSnafu)?
            .into())
    }

    async fn delete_room_streaming_target(
        &mut self,
        room_id: RoomId,
        streaming_target_id: StreamingTargetId,
    ) -> Result<()> {
        Ok(db::RoomStreamingTargetRecord::delete_by_id(
            &mut self.inner,
            room_id,
            streaming_target_id,
        )
        .await
        .context(DatabaseSnafu)?)
    }

    async fn replace_room_streaming_targets(
        &mut self,
        room_id: RoomId,
        streaming_targets: Vec<StreamingTarget>,
    ) -> Result<Vec<RoomStreamingTarget>> {
        Ok(
            override_room_streaming_targets(&mut self.inner, room_id, streaming_targets)
                .await
                .context(DatabaseSnafu)?,
        )
    }
}
