// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::rooms::RoomPassword;

/// Representation of an update to a room in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRoom {
    /// An optional password for the room.
    pub password: Option<Option<RoomPassword>>,

    /// A flag indicating that the wating room is enabled for this room.
    pub waiting_room: Option<bool>,

    /// A flag indicating that e2e encryption is enabled for this room.
    pub e2e_encryption: Option<bool>,
}

impl From<opentalk_db_storage::rooms::UpdateRoom> for UpdateRoom {
    fn from(
        opentalk_db_storage::rooms::UpdateRoom {
            password,
            waiting_room,
            e2e_encryption,
        }: opentalk_db_storage::rooms::UpdateRoom,
    ) -> Self {
        Self {
            password,
            waiting_room,
            e2e_encryption,
        }
    }
}

impl From<UpdateRoom> for opentalk_db_storage::rooms::UpdateRoom {
    fn from(
        UpdateRoom {
            password,
            waiting_room,
            e2e_encryption,
        }: UpdateRoom,
    ) -> Self {
        Self {
            password,
            waiting_room,
            e2e_encryption,
        }
    }
}
