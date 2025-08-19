// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{rooms::RoomId, time::Timestamp, users::UserId};

/// The representation of a new room invite that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRoomInvite {
    /// The id of the user who created the room invite.
    pub created_by: UserId,

    /// The id of the user who last updated the room invite.
    pub updated_by: UserId,

    /// The id of the room to which the invite belongs.
    pub room: RoomId,

    /// A flag that indicates whether the invite is active.
    pub active: bool,

    /// An optional expiration date.
    pub expiration: Option<Timestamp>,
}
