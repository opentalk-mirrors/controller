// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_inventory as inventory;
use opentalk_types_common::rooms::RoomPassword;

use crate::schema::rooms;

/// Diesel room struct for updates
///
/// Is used in update queries. None fields will be ignored on update queries
#[derive(Debug, AsChangeset)]
#[diesel(table_name = rooms)]
pub struct UpdateRoom {
    pub password: Option<Option<RoomPassword>>,
    pub waiting_room: Option<bool>,
    pub e2e_encryption: Option<bool>,
}

impl From<inventory::UpdateRoom> for UpdateRoom {
    fn from(
        inventory::UpdateRoom {
            password,
            waiting_room,
            e2e_encryption,
        }: inventory::UpdateRoom,
    ) -> Self {
        Self {
            password,
            waiting_room,
            e2e_encryption,
        }
    }
}
