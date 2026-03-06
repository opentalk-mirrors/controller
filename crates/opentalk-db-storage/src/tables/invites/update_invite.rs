// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    rooms::{RoomId, invite_codes::InviteCode},
    users::UserId,
};

use crate::{schema::invites, tables::invites::Invite};

/// Diesel invites struct
///
/// Represents a changeset of in invite
#[derive(Debug, AsChangeset)]
#[diesel(table_name = invites)]
pub struct UpdateInvite {
    pub updated_by: Option<UserId>,
    pub updated_at: Option<DateTime<Utc>>,
    pub room: Option<RoomId>,
    pub active: Option<bool>,
    pub expiration: Option<Option<DateTime<Utc>>>,
}

impl From<inventory::UpdateRoomInvite> for UpdateInvite {
    fn from(
        inventory::UpdateRoomInvite {
            updated_by,
            updated_at,
            room,
            active,
            expiration,
        }: inventory::UpdateRoomInvite,
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

impl UpdateInvite {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        room_id: RoomId,
        invite_code_id: InviteCode,
    ) -> Result<Invite> {
        let query = diesel::update(invites::table)
            .filter(
                invites::id
                    .eq(invite_code_id)
                    .and(invites::room.eq(room_id)),
            )
            .set(self)
            .returning(invites::all_columns);

        let invite = query.get_result(conn).await?;

        Ok(invite)
    }
}
