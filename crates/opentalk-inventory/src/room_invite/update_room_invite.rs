// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{rooms::RoomId, time::Timestamp, users::UserId};

/// Representation of an update to a [`super::RoomInvite`] in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRoomInvite {
    /// The id of the user last updated the invite.
    pub updated_by: Option<UserId>,

    /// The updated timestamp.
    pub updated_at: Option<Timestamp>,

    /// The id of the room to which the invite belongs.
    pub room: Option<RoomId>,

    /// A flag that indicates whether the invite is active.
    pub active: Option<bool>,

    /// An optional expiration date.
    pub expiration: Option<Option<Timestamp>>,
}
