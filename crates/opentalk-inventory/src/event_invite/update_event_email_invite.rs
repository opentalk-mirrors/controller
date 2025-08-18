// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::events::invites::EmailInviteRole;

/// Representation of an update to a room in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEventEmailInvite {
    /// The role of the invited person.
    pub role: Option<EmailInviteRole>,
}

impl From<opentalk_db_storage::events::email_invites::UpdateEventEmailInvite>
    for UpdateEventEmailInvite
{
    fn from(
        opentalk_db_storage::events::email_invites::UpdateEventEmailInvite{ role }: opentalk_db_storage::events::email_invites::UpdateEventEmailInvite,
    ) -> Self {
        Self { role }
    }
}

impl From<UpdateEventEmailInvite>
    for opentalk_db_storage::events::email_invites::UpdateEventEmailInvite
{
    fn from(UpdateEventEmailInvite { role }: UpdateEventEmailInvite) -> Self {
        Self { role }
    }
}
