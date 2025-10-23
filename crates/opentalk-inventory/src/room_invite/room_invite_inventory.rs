// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    rooms::{RoomId, invite_codes::InviteCode},
    time::Timestamp,
    users::UserId,
};

use super::{NewRoomInvite, RoomInvite, RoomInviteWithUsers, UpdateRoomInvite};
use crate::Result;

/// A trait for retrieving and storing room invite entities.
#[async_trait::async_trait]
pub trait RoomInviteInventory {
    /// Create a room invite.
    async fn create_room_invite(&mut self, invite: NewRoomInvite) -> Result<RoomInvite>;

    /// Get a room invite by the invite code.
    async fn get_room_invite(&mut self, invite_code: InviteCode) -> Result<RoomInvite>;

    /// Get all room invites.
    async fn get_all_room_invites(&mut self) -> Result<Vec<RoomInvite>>;

    /// Get a valid invite for a room.
    async fn get_valid_invite_for_room(&mut self, room_id: RoomId) -> Result<Option<RoomInvite>>;

    /// Get a valid invite for a room, or create one if none exists.
    ///
    /// If no invite is found for the room, a new invite will be created.
    /// The caller of this function must take care to create access rules
    /// because this crate does not have access to that functionality.
    async fn get_or_create_valid_invite_for_room(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
    ) -> Result<RoomInvite>;

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

    /// Update a room invite.
    async fn update_room_invite(
        &mut self,
        room_id: RoomId,
        invite_code: InviteCode,
        invite: UpdateRoomInvite,
    ) -> Result<RoomInvite>;

    /// Get the invite code
    async fn get_room_invites_with_room_inactive_or_expired_before(
        &mut self,
        expired_before: Timestamp,
    ) -> Result<Vec<(InviteCode, RoomId)>>;
}
