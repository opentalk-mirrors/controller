// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::Utc;
use opentalk_database::DatabaseError;
use opentalk_db_storage as db;
use opentalk_inventory::{
    NewRoomInvite, RoomInvite, RoomInviteInventory, RoomInviteWithUsers, UpdateRoomInvite,
};
use opentalk_types_common::{
    pagination::{ItemCount, Page, PageSize},
    rooms::{RoomId, invite_codes::InviteCode},
    time::Timestamp,
    users::UserId,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Error, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl RoomInviteInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn create_room_invite(&mut self, invite: NewRoomInvite) -> Result<RoomInvite> {
        Ok(
            db::queries::invites::create_room_invite(&mut self.inner, invite.into())
                .await
                .context(DatabaseSnafu)?
                .into(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invite(&mut self, invite_code: InviteCode) -> Result<RoomInvite> {
        match db::queries::invites::get_room_invite(&mut self.inner, invite_code).await {
            Ok(invite) => Ok(invite.into()),
            Err(DatabaseError::NotFound) => Err(Error::NotFound),
            Err(err) => Err(err).context(DatabaseSnafu)?,
        }
    }

    async fn get_all_room_invites(&mut self) -> Result<Vec<RoomInvite>> {
        Ok(db::queries::invites::get_all_invites(&mut self.inner)
            .await
            .context(DatabaseSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_valid_invite_for_room(&mut self, room_id: RoomId) -> Result<Option<RoomInvite>> {
        Ok(
            db::queries::invites::get_valid_invite_for_room(&mut self.inner, room_id, Utc::now())
                .await
                .context(DatabaseSnafu)?
                .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_or_create_valid_invite_for_room(
        &mut self,
        room_id: RoomId,
        user_id: UserId,
    ) -> Result<RoomInvite> {
        Ok(db::queries::invites::get_or_create_valid_invite_for_room(
            &mut self.inner,
            room_id,
            user_id,
        )
        .await
        .context(DatabaseSnafu)?
        .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_updated_by(&mut self, user_id: UserId) -> Result<Vec<RoomInvite>> {
        Ok(
            db::queries::invites::get_room_invites_updated_by(&mut self.inner, user_id)
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_paginated_with_creator_and_updater(
        &mut self,
        room_id: RoomId,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<RoomInviteWithUsers>, ItemCount)> {
        let (invites, overall) =
            db::queries::invites::get_room_invites_paginated_with_creator_and_updater(
                &mut self.inner,
                room_id,
                limit,
                page,
            )
            .await
            .context(DatabaseSnafu)?;
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
            db::queries::invites::get_room_invite_with_creator_and_updater(
                &mut self.inner,
                invite_code,
            )
            .await
            .context(DatabaseSnafu)?;
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
        invite: UpdateRoomInvite,
    ) -> Result<RoomInvite> {
        Ok(db::queries::invites::update_room_invite(
            &mut self.inner,
            invite.into(),
            room_id,
            invite_code,
        )
        .await
        .context(DatabaseSnafu)?
        .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_room_invites_with_room_inactive_or_expired_before(
        &mut self,
        expired_before: Timestamp,
    ) -> Result<Vec<(InviteCode, RoomId)>> {
        Ok(db::queries::invites::get_inactive_or_expired_before_invite(
            &mut self.inner,
            expired_before.into(),
        )
        .await
        .context(DatabaseSnafu)?)
    }
}
