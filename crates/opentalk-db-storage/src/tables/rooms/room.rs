// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    rooms::{RoomId, RoomPassword},
    tenants::TenantId,
    users::UserId,
};

use crate::{schema::rooms, tables::rooms::SerialRoomId};

/// Diesel room struct
///
/// Is used as a result in various queries. Represents a room column
#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = rooms)]
pub struct Room {
    pub id: RoomId,
    pub id_serial: SerialRoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub password: Option<RoomPassword>,
    pub waiting_room: bool,
    pub tenant_id: TenantId,
    pub e2e_encryption: bool,
}

impl From<Room> for inventory::Room {
    fn from(
        Room {
            id,
            id_serial,
            created_by,
            created_at,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: Room,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}

impl From<inventory::Room> for Room {
    fn from(
        inventory::Room {
            id,
            id_serial,
            created_by,
            created_at,
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }: inventory::Room,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            password,
            waiting_room,
            tenant_id,
            e2e_encryption,
        }
    }
}
