// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, prelude::*};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::events::{EventId, invites::EmailInviteRole};

use crate::{schema::event_email_invites, tables::event_email_invites::EventEmailInvite};

#[derive(AsChangeset)]
#[diesel(table_name = event_email_invites)]
pub struct UpdateEventEmailInvite {
    pub role: Option<EmailInviteRole>,
}

impl From<inventory::UpdateEventEmailInvite> for UpdateEventEmailInvite {
    fn from(inventory::UpdateEventEmailInvite { role }: inventory::UpdateEventEmailInvite) -> Self {
        Self { role }
    }
}

impl UpdateEventEmailInvite {
    /// Apply the update to the invite where `email` is the invitee's email address
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        email: &str,
        event_id: EventId,
    ) -> Result<EventEmailInvite> {
        // TODO: Check if the update actually applied a change (see comments in fn `apply` of `UpdateEventInvite`)
        let query = diesel::update(event_email_invites::table)
            .filter(
                event_email_invites::event_id
                    .eq(event_id)
                    .and(event_email_invites::email.eq(email)),
            )
            .set(self)
            .returning(event_email_invites::all_columns);

        let event_invite = query.get_result(conn).await?;

        Ok(event_invite)
    }
}
