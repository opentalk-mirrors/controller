// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{rooms::RoomPassword, tenants::TenantId, users::UserId};

use crate::{schema::rooms, tables::rooms::Room};

/// Diesel insertable room struct
///
/// Represents fields that have to be provided on room insertion.
#[derive(Debug, Insertable)]
#[diesel(table_name = rooms)]
pub struct NewRoom {
    pub created_by: UserId,
    pub password: Option<RoomPassword>,
    pub waiting_room: bool,
    pub tenant_id: TenantId,
    pub e2e_encryption: bool,
}

impl From<inventory::NewRoom> for NewRoom {
    fn from(
        inventory::NewRoom {
            created_by,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: inventory::NewRoom,
    ) -> Self {
        Self {
            created_by,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}

impl NewRoom {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<Room> {
        let room = self.insert_into(rooms::table).get_result(conn).await?;

        Ok(room)
    }
}
