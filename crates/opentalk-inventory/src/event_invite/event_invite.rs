// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    events::{
        EventId,
        invites::{EventInviteStatus, InviteRole},
    },
    time::Timestamp,
    users::UserId,
};

use super::EventInviteId;

/// The representation of an event invite in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInvite {
    /// The id of the event invite.
    pub id: EventInviteId,

    /// The id of the event.
    pub event_id: EventId,

    /// The id of the invitee.
    pub invitee: UserId,

    /// The id of the user who created the event invite.
    pub created_by: UserId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The status of the event invite.
    pub status: EventInviteStatus,

    /// The role of the invited user.
    pub role: InviteRole,
}

impl From<opentalk_db_storage::events::EventInvite> for EventInvite {
    fn from(
        opentalk_db_storage::events::EventInvite {
            id,
            event_id,
            invitee,
            created_by,
            created_at,
            status,
            role,
        }: opentalk_db_storage::events::EventInvite,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            invitee,
            created_by,
            created_at: created_at.into(),
            status,
            role,
        }
    }
}

impl From<EventInvite> for opentalk_db_storage::events::EventInvite {
    fn from(
        EventInvite {
            id,
            event_id,
            invitee,
            created_by,
            created_at,
            status,
            role,
        }: EventInvite,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            invitee,
            created_by,
            created_at: created_at.into(),
            status,
            role,
        }
    }
}
