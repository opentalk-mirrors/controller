// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::Insertable;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventId, invites::EmailInviteRole},
    users::UserId,
};

use crate::{schema::event_email_invites, tables::event_email_invites::EventEmailInvite};

#[derive(Insertable)]
#[diesel(table_name = event_email_invites)]
pub struct NewEventEmailInvite {
    pub event_id: EventId,
    pub email: String,
    pub role: EmailInviteRole,
    pub created_by: UserId,
}

impl From<inventory::NewEventEmailInvite> for NewEventEmailInvite {
    fn from(
        inventory::NewEventEmailInvite {
            event_id,
            email,
            role,
            created_by,
        }: inventory::NewEventEmailInvite,
    ) -> Self {
        Self {
            event_id,
            email,
            role,
            created_by,
        }
    }
}

impl NewEventEmailInvite {
    /// Tries to insert the EventEmailInvite into the database
    ///
    /// When yielding a unique key violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<EventEmailInvite>> {
        let query = self.insert_into(event_email_invites::table);

        let result = query.get_result(conn).await;

        match result {
            Ok(event_email_invites) => Ok(Some(event_email_invites)),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..,
            )) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
