// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_database::DatabaseError;
use opentalk_db_storage::sip_configs as db;
use opentalk_inventory::{
    NewRoomSipConfig, Room, RoomSipConfig, RoomSipConfigInventory, UpdateRoomSipConfig,
};
use opentalk_types_common::{call_in::CallInId, rooms::RoomId};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl RoomSipConfigInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn get_room_sip_config(&mut self, room_id: RoomId) -> Result<Option<RoomSipConfig>> {
        match db::SipConfig::get_by_room(&mut self.inner, room_id).await {
            Ok(sip_config) => Ok(Some(sip_config.into())),
            Err(DatabaseError::NotFound) => Ok(None),
            Err(e) => Err(e).context(DatabaseSnafu)?,
        }
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_sip_config_with_room(
        &mut self,
        call_in_id: CallInId,
    ) -> Result<Option<(RoomSipConfig, Room)>> {
        Ok(db::SipConfig::get_with_room(&mut self.inner, &call_in_id)
            .await
            .context(DatabaseSnafu)?
            .map(|(sip_config, room)| (sip_config.into(), room.into())))
    }

    #[tracing::instrument(err, skip_all)]
    async fn create_room_sip_config(
        &mut self,
        sip_config: NewRoomSipConfig,
    ) -> Result<RoomSipConfig> {
        Ok(db::NewSipConfig::from(sip_config)
            .insert(&mut self.inner)
            .await
            .context(DatabaseSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_room_sip_config(
        &mut self,
        room_id: RoomId,
        sip_config: UpdateRoomSipConfig,
    ) -> Result<Option<RoomSipConfig>> {
        Ok(db::UpdateSipConfig::from(sip_config)
            .apply(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?
            .map(Into::into))
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_room_sip_config(&mut self, room_id: RoomId) -> Result<()> {
        Ok(db::SipConfig::delete_by_room(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?)
    }
}
