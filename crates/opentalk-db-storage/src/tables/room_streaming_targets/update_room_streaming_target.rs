// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    rooms::RoomId,
    streaming::{StreamingKind, StreamingTargetId},
};

use crate::{
    schema::room_streaming_targets, tables::room_streaming_targets::RoomStreamingTargetRecord,
};

/// Diesel streaming target struct
///
/// Represents a changeset of in invite
#[derive(Debug, AsChangeset)]
#[diesel(table_name = room_streaming_targets)]
pub struct UpdateRoomStreamingTarget {
    pub name: Option<String>,
    pub kind: Option<StreamingKind>,
    pub streaming_endpoint: Option<String>,
    pub streaming_key: Option<String>,
    pub public_url: Option<String>,
}

impl From<inventory::UpdateRoomStreamingTarget> for UpdateRoomStreamingTarget {
    fn from(
        inventory::UpdateRoomStreamingTarget {
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }: inventory::UpdateRoomStreamingTarget,
    ) -> Self {
        Self {
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }
    }
}

impl UpdateRoomStreamingTarget {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        room_id: RoomId,
        streaming_target_id: StreamingTargetId,
    ) -> Result<RoomStreamingTargetRecord> {
        let query = diesel::update(room_streaming_targets::table)
            .filter(room_streaming_targets::id.eq(streaming_target_id))
            .filter(room_streaming_targets::room_id.eq(room_id))
            .set(self)
            .returning(room_streaming_targets::all_columns);

        let invite = query.get_result(conn).await?;

        Ok(invite)
    }
}
