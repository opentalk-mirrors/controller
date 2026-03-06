// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{rooms::RoomId, users::UserId};

use crate::{schema::invites, tables::invites::Invite};

/// Diesel invites struct
///
/// Represents a new invite in the database
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = invites)]
pub struct NewInvite {
    pub created_by: UserId,
    pub updated_by: UserId,
    pub room: RoomId,
    pub active: bool,
    pub expiration: Option<DateTime<Utc>>,
}

impl From<inventory::NewRoomInvite> for NewInvite {
    fn from(
        inventory::NewRoomInvite {
            created_by,
            updated_by,
            room,
            active,
            expiration,
        }: inventory::NewRoomInvite,
    ) -> Self {
        Self {
            created_by,
            updated_by,
            room,
            active,
            expiration: expiration.map(Into::into),
        }
    }
}

impl NewInvite {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<Invite> {
        let query = diesel::insert_into(invites::table).values(self);

        let invite = query.get_result(conn).await?;

        Ok(invite)
    }
}
