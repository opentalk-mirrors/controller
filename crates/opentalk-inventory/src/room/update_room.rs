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
