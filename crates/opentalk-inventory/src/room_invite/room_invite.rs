// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    rooms::{RoomId, invite_codes::InviteCode},
    time::Timestamp,
    users::UserId,
};

/// The representation of a room invite in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomInvite {
    /// The invite code.
    pub invite_code: InviteCode,

    /// The serial id of the user.
    pub id_serial: i64,

    /// The id of the user who created the invite.
    pub created_by: UserId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The id of the user who updated the invite.
    pub updated_by: UserId,

    /// The updated timestamp.
    pub updated_at: Timestamp,

    /// The id of the room to which the invite belongs.
    pub room: RoomId,

    /// A flag indicating whether the invite is active.
    pub active: bool,

    /// An optional expiry timestmamp.
    pub expiration: Option<Timestamp>,
}

impl From<opentalk_db_storage::invites::Invite> for RoomInvite {
    fn from(
        opentalk_db_storage::invites::Invite {
            id,
            id_serial,
            created_by,
            created_at,
            updated_by,
            updated_at,
            room,
            active,
            expiration,
        }: opentalk_db_storage::invites::Invite,
    ) -> Self {
        Self {
            invite_code: id,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            updated_by,
            updated_at: updated_at.into(),
            room,
            active,
            expiration: expiration.map(Into::into),
        }
    }
}

impl From<RoomInvite> for opentalk_db_storage::invites::Invite {
    fn from(
        RoomInvite {
            invite_code,
            id_serial,
            created_by,
            created_at,
            updated_by,
            updated_at,
            room,
            active,
            expiration,
        }: RoomInvite,
    ) -> Self {
        Self {
            id: invite_code,
            id_serial: id_serial.into(),
            created_by,
            created_at: created_at.into(),
            updated_by,
            updated_at: updated_at.into(),
            room,
            active,
            expiration: expiration.map(Into::into),
        }
    }
}
