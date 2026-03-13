// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, Identifiable, QueryDsl, Queryable, prelude::*};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    call_in::{CallInId, CallInPassword},
    rooms::RoomId,
};

use crate::{
    schema::{rooms, sip_configs},
    tables::rooms::Room,
};

/// Diesel SipConfig struct
#[derive(Debug, Clone, Queryable, Identifiable)]
pub struct SipConfig {
    pub id: i64,
    pub room: RoomId,
    pub sip_id: CallInId,
    pub password: CallInPassword,
    pub lobby: bool,
}

impl From<SipConfig> for inventory::RoomSipConfig {
    fn from(
        SipConfig {
            id,
            room,
            sip_id,
            password,
            lobby,
        }: SipConfig,
    ) -> Self {
        Self {
            id,
            room,
            sip_id,
            password,
            lobby,
        }
    }
}

impl From<inventory::RoomSipConfig> for SipConfig {
    fn from(
        inventory::RoomSipConfig {
            id,
            room,
            sip_id,
            password,
            lobby,
        }: inventory::RoomSipConfig,
    ) -> Self {
        Self {
            id,
            room,
            sip_id,
            password,
            lobby,
        }
    }
}

impl SipConfig {
    /// Get the sip config for the specified sip_id
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, sip_id: CallInId) -> Result<Option<SipConfig>> {
        let query = sip_configs::table.filter(sip_configs::sip_id.eq(&sip_id));
        let sip_config = query.get_result(conn).await.optional()?;

        Ok(sip_config)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_with_room(
        conn: &mut DbConnection,
        sip_id: &CallInId,
    ) -> Result<Option<(SipConfig, Room)>> {
        let query = sip_configs::table
            .filter(sip_configs::sip_id.eq(sip_id))
            .inner_join(rooms::table);

        let result: Option<(SipConfig, Room)> = query.get_result(conn).await.optional()?;

        Ok(result)
    }

    /// Get the sip config for the specified room
    #[tracing::instrument(err, skip_all)]
    pub async fn get_by_room(conn: &mut DbConnection, room_id: RoomId) -> Result<SipConfig> {
        let query = sip_configs::table.filter(sip_configs::room.eq(&room_id));
        let sip_config = query.get_result(conn).await?;

        Ok(sip_config)
    }

    /// Delete the sip config for the specified room
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        let query = diesel::delete(sip_configs::table.filter(sip_configs::room.eq(&room_id)));

        query.execute(conn).await?;

        Ok(())
    }

    pub async fn delete(&self, conn: &mut DbConnection) -> Result<()> {
        Self::delete_by_room(conn, self.room).await
    }
}
