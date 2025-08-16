// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::events::invites::{EventInviteStatus, InviteRole};

/// Representation of an update to an [`super::EventInvite`] in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEventInvite {
    /// Update the status of the event invite.
    pub status: Option<EventInviteStatus>,

    /// Update the role of the event invite.
    pub role: Option<InviteRole>,
}

impl From<opentalk_db_storage::events::UpdateEventInvite> for UpdateEventInvite {
    fn from(
        opentalk_db_storage::events::UpdateEventInvite{ status, role }: opentalk_db_storage::events::UpdateEventInvite,
    ) -> Self {
        Self { status, role }
    }
}

impl From<UpdateEventInvite> for opentalk_db_storage::events::UpdateEventInvite {
    fn from(UpdateEventInvite { status, role }: UpdateEventInvite) -> Self {
        Self { status, role }
    }
}
