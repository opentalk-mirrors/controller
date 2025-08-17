// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use crate::{RoomInvite, User};

/// A room invite with the users that created and updated it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomInviteWithUsers {
    /// The room invite.
    pub invite: RoomInvite,

    /// The user who created the invite.
    pub created_by: User,

    /// The user who last updated the invite.
    pub updated_by: User,
}

impl RoomInviteWithUsers {
    /// Create a new [`RoomInviteWithUsers`].
    pub fn new(invite: RoomInvite, created_by: User, updated_by: User) -> Self {
        Self {
            invite,
            created_by,
            updated_by,
        }
    }
}
