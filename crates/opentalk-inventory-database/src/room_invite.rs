// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::Utc;
use opentalk_db_storage::invites::{self as db, UpdateInvite};
use opentalk_inventory::{
    NewRoomInvite, RoomInvite, RoomInviteInventory, RoomInviteWithUsers, error::StorageBackendSnafu,
};
use opentalk_types_common::{
    rooms::{RoomId, invite_codes::InviteCode},
    time::Timestamp,
    users::UserId,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl RoomInviteInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn create_room_invite(&mut self, invite: NewRoomInvite) -> Result<RoomInvite> {
        Ok(db::NewInvite::from(invite)
            .insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invite(&mut self, invite_code: InviteCode) -> Result<RoomInvite> {
        Ok(db::Invite::get(&mut self.inner, invite_code)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    async fn get_all_room_invites(&mut self) -> Result<Vec<RoomInvite>> {
        Ok(db::Invite::get_all(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_valid_invite_for_room(&mut self, room_id: RoomId) -> Result<Option<RoomInvite>> {
        Ok(
            db::Invite::get_valid_for_room(&mut self.inner, room_id, Utc::now())
                .await
                .context(StorageBackendSnafu)?
                .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_or_create_valid_invite_for_room(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
    ) -> Result<RoomInvite> {
        Ok(
            db::Invite::get_valid_or_create_for_room(&mut self.inner, room_id, user_id)
                .await
                .context(StorageBackendSnafu)?
                .into(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_updated_by(&mut self, user_id: UserId) -> Result<Vec<RoomInvite>> {
        Ok(db::Invite::get_updated_by(&mut self.inner, user_id)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_paginated_with_creator_and_updater(
        &mut self,
        room_id: RoomId,
        limit: i64,
        page: i64,
    ) -> Result<(Vec<RoomInviteWithUsers>, i64)> {
        let (invites, overall) = db::Invite::get_all_for_room_with_users_paginated(
            &mut self.inner,
            room_id,
            limit,
            page,
        )
        .await
        .context(StorageBackendSnafu)?;
        Ok((
            invites
                .into_iter()
                .map(|(invite, created_by, updated_by)| {
                    RoomInviteWithUsers::new(invite.into(), created_by.into(), updated_by.into())
                })
                .collect(),
            overall,
        ))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invite_with_creator_and_updater(
        &mut self,
        invite_code: InviteCode,
    ) -> Result<RoomInviteWithUsers> {
        let (invite, created_by, updated_by) =
            db::Invite::get_with_users(&mut self.inner, invite_code)
                .await
                .context(StorageBackendSnafu)?;
        Ok(RoomInviteWithUsers::new(
            invite.into(),
            created_by.into(),
            updated_by.into(),
        ))
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_room_invite(
        &mut self,
        room_id: RoomId,
        invite_code: InviteCode,
        invite: UpdateInvite,
    ) -> Result<RoomInvite> {
        Ok(invite
            .apply(&mut self.inner, room_id, invite_code)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_with_room_inactive_or_expired_before(
        &mut self,
        expired_before: Timestamp,
    ) -> Result<Vec<(InviteCode, RoomId)>> {
        db::Invite::get_inactive_or_expired_before(&mut self.inner, expired_before.into())
            .await
            .context(StorageBackendSnafu)
    }
}
