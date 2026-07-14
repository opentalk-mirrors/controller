// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    rooms::{RoomId, invite_codes::InviteCode},
    time::Timestamp,
    users::UserId,
};

use super::{RoomInvite, RoomInviteWithUsers};
use crate::Result;

/// A trait for retrieving and storing room invite entities.
#[async_trait::async_trait]
pub trait RoomInviteInventory {
    /// Get a room invite by the invite code.
    async fn get_room_invite(&mut self, invite_code: InviteCode) -> Result<RoomInvite>;

    /// Get all room invites.
    async fn get_all_room_invites(&mut self) -> Result<Vec<RoomInvite>>;

    /// Get all room invites updated by a specific user.
    async fn get_room_invites_updated_by(&mut self, user_id: UserId) -> Result<Vec<RoomInvite>>;

    /// Get all room invites with the creator and updater users.
    ///
    /// Returns a tuple with:
    /// - `Vec<(Invite, CreatedByUser, UpdatedByUser)>` - A Vec of invites along with the users that created and updated the invite
    /// - `i64`: the total number of records.
    async fn get_room_invites_paginated_with_creator_and_updater(
        &mut self,
        room_id: RoomId,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<RoomInviteWithUsers>, ItemCount)>;

    /// Get a room invite with the creator and updater user.
    async fn get_room_invite_with_creator_and_updater(
        &mut self,
        invite_code: InviteCode,
    ) -> Result<RoomInviteWithUsers>;

    /// Get the invite code
    async fn get_room_invites_with_room_inactive_or_expired_before(
        &mut self,
        expired_before: Timestamp,
    ) -> Result<Vec<(InviteCode, RoomId)>>;
}
