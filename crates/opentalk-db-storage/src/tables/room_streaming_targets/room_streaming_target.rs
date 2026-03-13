// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, Identifiable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    self as types,
    rooms::RoomId,
    streaming::{
        StreamingKey, StreamingKind, StreamingTarget, StreamingTargetId, StreamingTargetKind,
    },
};
use opentalk_types_signaling_recording::{StreamKindSecret, StreamStatus, StreamTargetSecret};
use snafu::Snafu;
use url::Url;

use crate::{schema::room_streaming_targets, tables::rooms::Room};

#[derive(Debug, Queryable, Identifiable, Associations, Insertable)]
#[diesel(belongs_to(Room, foreign_key = room_id))]
#[diesel(table_name = room_streaming_targets)]
pub struct RoomStreamingTarget {
    pub id: StreamingTargetId,
    pub room_id: RoomId,
    pub name: String,
    pub kind: StreamingKind,
    pub streaming_endpoint: String,
    pub streaming_key: StreamingKey,
    pub public_url: String,
}

impl From<inventory::RoomStreamingTargetRecord> for RoomStreamingTarget {
    fn from(
        inventory::RoomStreamingTargetRecord {
            id,
            room_id,
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }: inventory::RoomStreamingTargetRecord,
    ) -> Self {
        Self {
            id,
            room_id,
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }
    }
}

impl From<RoomStreamingTarget> for inventory::RoomStreamingTargetRecord {
    fn from(
        RoomStreamingTarget {
            id,
            room_id,
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }: RoomStreamingTarget,
    ) -> Self {
        Self {
            id,
            room_id,
            name,
            kind,
            streaming_endpoint,
            streaming_key,
            public_url,
        }
    }
}

impl RoomStreamingTarget {
    /// Retrieve a single streaming target
    #[tracing::instrument(err, skip_all)]
    pub async fn get(
        conn: &mut DbConnection,
        streaming_target_id: StreamingTargetId,
        room_id: RoomId,
    ) -> Result<RoomStreamingTarget> {
        let streaming_target = room_streaming_targets::table
            .filter(room_streaming_targets::id.eq(streaming_target_id))
            .filter(room_streaming_targets::room_id.eq(room_id))
            .first(conn)
            .await?;

        Ok(streaming_target)
    }

    /// Retrieve all streaming targets
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_room(
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Vec<RoomStreamingTarget>> {
        let streaming_targets = room_streaming_targets::table
            .filter(room_streaming_targets::room_id.eq(room_id))
            .load(conn)
            .await?;

        Ok(streaming_targets)
    }

    /// Delete a streaming target using the given room & streaming target id
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(
        conn: &mut DbConnection,
        room_id: RoomId,
        streaming_target_id: StreamingTargetId,
    ) -> Result<()> {
        let _ = diesel::delete(
            room_streaming_targets::table
                .filter(room_streaming_targets::id.eq(streaming_target_id))
                .filter(room_streaming_targets::room_id.eq(room_id)),
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Delete all streaming targets that are associated with a specific room
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_room_id(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        let _ = diesel::delete(
            room_streaming_targets::table.filter(room_streaming_targets::room_id.eq(room_id)),
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

impl TryFrom<RoomStreamingTarget> for types::streaming::RoomStreamingTarget {
    type Error = DatabaseError;

    fn try_from(record: RoomStreamingTarget) -> Result<Self, Self::Error> {
        let kind = match record.kind {
            StreamingKind::Custom => StreamingTargetKind::Custom {
                streaming_endpoint: Url::parse(&record.streaming_endpoint)?,
                streaming_key: record.streaming_key,
                public_url: Url::parse(&record.public_url)?,
            },
        };

        Ok(Self {
            id: record.id,
            streaming_target: StreamingTarget {
                name: record.name,
                kind,
            },
        })
    }
}

#[derive(Debug, Snafu)]
pub enum StreamTargetConversionError {
    #[snafu(display("Parsing the url failed, because {target} is not a valid URL"))]
    WrongUrl { target: String },
}

impl TryFrom<RoomStreamingTarget> for StreamTargetSecret {
    type Error = StreamTargetConversionError;

    fn try_from(value: RoomStreamingTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            kind: StreamKindSecret::Livestream(match value.kind {
                StreamingKind::Custom => StreamingTargetKind::Custom {
                    streaming_endpoint: value.streaming_endpoint.parse().map_err(|_| {
                        StreamTargetConversionError::WrongUrl {
                            target: value.streaming_endpoint,
                        }
                    })?,
                    streaming_key: value.streaming_key,
                    public_url: value.public_url.parse().map_err(|_| {
                        StreamTargetConversionError::WrongUrl {
                            target: value.public_url,
                        }
                    })?,
                },
            }),
            status: StreamStatus::Inactive,
        })
    }
}
