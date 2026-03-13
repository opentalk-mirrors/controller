// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{call_in::CallInPassword, rooms::RoomId};

use crate::{schema::sip_configs, tables::sip_configs::SipConfig};

/// Diesel struct to modify a SipConfig
#[derive(Debug, AsChangeset)]
#[diesel(table_name = sip_configs)]
pub struct UpdateSipConfig {
    pub password: Option<CallInPassword>,
    pub enable_lobby: Option<bool>,
}

impl From<inventory::UpdateRoomSipConfig> for UpdateSipConfig {
    fn from(
        inventory::UpdateRoomSipConfig {
            password,
            enable_lobby,
        }: inventory::UpdateRoomSipConfig,
    ) -> Self {
        Self {
            password,
            enable_lobby,
        }
    }
}

impl UpdateSipConfig {
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Option<SipConfig>> {
        let query =
            diesel::update(sip_configs::table.filter(sip_configs::room.eq(&room_id))).set(self);

        let config = query.get_result(conn).await.optional()?;

        Ok(config)
    }
}
