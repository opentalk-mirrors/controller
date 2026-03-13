// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    call_in::{CallInId, CallInPassword},
    rooms::RoomId,
};

use crate::{schema::sip_configs, tables::sip_configs::SipConfig};

/// Diesel insertable SipConfig struct
///
/// Represents fields that have to be provided on insertion.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = sip_configs)]
pub struct NewSipConfig {
    pub room: RoomId,
    pub sip_id: CallInId,
    pub password: CallInPassword,
    pub enable_lobby: bool,
}

impl From<inventory::NewRoomSipConfig> for NewSipConfig {
    fn from(
        inventory::NewRoomSipConfig {
            room,
            sip_id,
            password,
            enable_lobby,
        }: inventory::NewRoomSipConfig,
    ) -> Self {
        Self {
            room,
            sip_id,
            password,
            enable_lobby,
        }
    }
}

impl NewSipConfig {
    fn re_generate_id(&mut self) {
        self.sip_id = CallInId::generate();
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn insert(mut self, conn: &mut DbConnection) -> Result<SipConfig> {
        for _ in 0..3 {
            let query = self.clone().insert_into(sip_configs::table);

            let config = match query.get_result(conn).await {
                Ok(config) => config,
                Err(diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                )) => {
                    self.re_generate_id();
                    continue;
                }
                Err(e) => return Err(e.into()),
            };

            return Ok(config);
        }

        Err(DatabaseError::Custom {
            message: format!(
                "Failed to insert new sip config for room {} 3 times (collision)",
                self.room
            ),
        })
    }
}
