// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use opentalk_inventory as inventory;
use opentalk_types_common::{rooms::RoomId, users::UserId};

use crate::schema::invites;

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
