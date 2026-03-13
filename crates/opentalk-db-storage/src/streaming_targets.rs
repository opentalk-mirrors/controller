// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_types_common::{
    rooms::RoomId,
    streaming::{RoomStreamingTarget, StreamingTarget, StreamingTargetKind},
};
use snafu::Report;

use crate::schema::room_streaming_targets;

pub use crate::tables::room_streaming_targets::{
    RoomStreamingTargetNew, RoomStreamingTargetRecord, UpdateRoomStreamingTarget,
};

pub async fn get_room_streaming_targets(
    conn: &mut DbConnection,
    room_id: RoomId,
) -> Result<Vec<RoomStreamingTarget>> {
    let streaming_targets = RoomStreamingTargetRecord::get_all_for_room(conn, room_id).await?;

    let room_streaming_targets = streaming_targets
        .into_iter()
        .map(|st| {
            let streaming_endpoint = st.streaming_endpoint.parse().map_err(|err| {
                log::warn!(
                    "Failed to parse streaming endpoint: {}",
                    Report::from_error(err)
                );
                DatabaseError::Custom {
                    message: "Inconsistent data".to_string(),
                }
            })?;
            let public_url = st.public_url.parse().map_err(|err| {
                log::warn!(
                    "Invalid public url entry in db: {}",
                    Report::from_error(err)
                );
                DatabaseError::Custom {
                    message: "Inconsistent data".to_string(),
                }
            })?;

            let room_streaming_target = RoomStreamingTarget {
                id: st.id,
                streaming_target: StreamingTarget {
                    name: st.name,
                    kind: StreamingTargetKind::Custom {
                        streaming_endpoint,
                        streaming_key: st.streaming_key,
                        public_url,
                    },
                },
            };

            Ok(room_streaming_target)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(room_streaming_targets)
}

pub async fn insert_room_streaming_target(
    conn: &mut DbConnection,
    room_id: RoomId,
    streaming_target: StreamingTarget,
) -> Result<RoomStreamingTarget> {
    let streaming_target_record = RoomStreamingTargetNew::from_streaming_target_kind(
        streaming_target.kind.clone(),
        room_id,
        streaming_target.name.clone(),
    )
    .insert(conn)
    .await?;

    let room_streaming_target = RoomStreamingTarget {
        id: streaming_target_record.id,
        streaming_target,
    };
    Ok(room_streaming_target)
}

pub async fn override_room_streaming_targets(
    conn: &mut DbConnection,
    room_id: RoomId,
    streaming_targets: Vec<StreamingTarget>,
) -> Result<Vec<RoomStreamingTarget>> {
    conn.transaction(|conn| {
        async move {
            // Delete existing records by room_id
            RoomStreamingTargetRecord::delete_by_room_id(conn, room_id).await?;

            let new_records: Vec<RoomStreamingTargetNew> = streaming_targets
                .into_iter()
                .map(|streaming_target| {
                    RoomStreamingTargetNew::from_streaming_target_kind(
                        streaming_target.kind,
                        room_id,
                        streaming_target.name,
                    )
                })
                .collect();

            // Insert new records and fetch the resulting records
            let inserted_records: Vec<RoomStreamingTargetRecord> =
                diesel::insert_into(room_streaming_targets::table)
                    .values(&new_records)
                    .get_results(conn)
                    .await?;

            Ok(inserted_records)
        }
        .scope_boxed()
    })
    .await
    .and_then(|inserted_records| {
        // Transform inserted_records into RoomStreamingTarget
        inserted_records
            .into_iter()
            .map(|record| record.try_into())
            .collect::<Result<Vec<_>, DatabaseError>>()
    })
}
