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

impl From<opentalk_db_storage::invites::UpdateInvite> for UpdateRoomInvite {
    fn from(
        opentalk_db_storage::invites::UpdateInvite {
            updated_by,
            updated_at,
            room,
            active,
            expiration,
        }: opentalk_db_storage::invites::UpdateInvite,
    ) -> Self {
        Self {
            updated_by,
            updated_at: updated_at.map(Into::into),
            room,
            active,
            expiration: expiration.map(|e| e.map(Into::into)),
        }
    }
}

impl From<UpdateRoomInvite> for opentalk_db_storage::invites::UpdateInvite {
    fn from(
        UpdateRoomInvite {
            updated_by,
            updated_at,
            room,
            active,
            expiration,
        }: UpdateRoomInvite,
    ) -> Self {
        Self {
            updated_by,
            updated_at: updated_at.map(Into::into),
            room,
            active,
            expiration: expiration.map(|e| e.map(Into::into)),
        }
    }
}
