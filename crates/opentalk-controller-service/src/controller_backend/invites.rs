// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::AuthorizationChange;
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{
    NewRoomInvite, RoomInvite, RoomInviteWithUsers, UpdateRoomInvite, utils::is_invite_valid,
};
use opentalk_types_api_v1::{
    error::ApiError,
    pagination::PagePaginationQuery,
    rooms::by_room_id::invites::{
        GetRoomsInvitesResponseBody, InviteResource, PostInviteRequestBody,
        PostInviteVerifyRequestBody, PostInviteVerifyResponseBody, PutInviteRequestBody,
    },
    users::PublicUserProfile,
};
use opentalk_types_common::{
    pagination::ItemCount,
    rooms::{RoomIdOrAlias, invite_codes::InviteCode},
    time::Timestamp,
};

use super::{verify_invite_read, verify_invite_write};
use crate::{ControllerBackend, ToUserProfile};

impl ControllerBackend {
    pub(crate) async fn create_invite(
        &self,
        current_user: RequestUser,
        room_id_or_alias: RoomIdOrAlias,
        new_invite: PostInviteRequestBody,
    ) -> Result<InviteResource, CaptureApiError> {
        let settings = self.settings_provider.get();

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id_or_alias).await?;
        let tariff = self.get_tariff_for_user(room.created_by).await?;
        verify_invite_write(&tariff, &room)?;

        let invite = inventory
            .create_room_invite(NewRoomInvite {
                active: true,
                created_by: current_user.id,
                updated_by: current_user.id,
                room: room.id,
                expiration: new_invite.expiration.map(Into::into),
            })
            .await?;

        self.authorizer
            .apply_change(&AuthorizationChange::AddInviteCodeToRoom {
                room: room.id,
                invite_code: invite.invite_code,
                expiration: invite.expiration,
            })
            .await
            .map_err(|e| {
                log::error!("Could not apply changes in the authorization database: {e:?}");
                ApiError::internal()
            })?;

        let created_by = current_user.to_public_user_profile(&settings);
        let updated_by = current_user.to_public_user_profile(&settings);

        let invite = invite.into_invite_resource(created_by, updated_by);

        Ok(invite)
    }

    pub(crate) async fn get_invites(
        &self,
        room_id_or_alias: RoomIdOrAlias,
        pagination: &PagePaginationQuery,
    ) -> Result<(GetRoomsInvitesResponseBody, ItemCount), CaptureApiError> {
        let settings = self.settings_provider.get();

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id_or_alias).await?;
        let tariff = self.get_tariff_for_user(room.created_by).await?;
        verify_invite_read(&tariff, &room)?;

        let (invites_with_users, total_invites) = inventory
            .get_room_invites_paginated_with_creator_and_updater(
                room.id,
                pagination.per_page,
                pagination.page,
            )
            .await?;

        let invites = invites_with_users
            .into_iter()
            .map(
                |RoomInviteWithUsers {
                     invite,
                     created_by,
                     updated_by,
                 }| {
                    let created_by = created_by.to_public_user_profile(&settings);
                    let updated_by = updated_by.to_public_user_profile(&settings);

                    invite.into_invite_resource(created_by, updated_by)
                },
            )
            .collect::<Vec<InviteResource>>();

        Ok((GetRoomsInvitesResponseBody(invites), total_invites))
    }

    pub(crate) async fn get_invite(
        &self,
        room_id_or_alias: RoomIdOrAlias,
        invite_code: InviteCode,
    ) -> Result<InviteResource, CaptureApiError> {
        let settings = self.settings_provider.get();

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id_or_alias).await?;
        let tariff = self.get_tariff_for_user(room.created_by).await?;
        verify_invite_read(&tariff, &room)?;

        let RoomInviteWithUsers {
            invite,
            created_by,
            updated_by,
        } = inventory
            .get_room_invite_with_creator_and_updater(invite_code)
            .await?;

        if invite.room != room.id {
            return Err(ApiError::not_found().into());
        }

        let created_by = created_by.to_public_user_profile(&settings);
        let updated_by = updated_by.to_public_user_profile(&settings);

        Ok(invite.into_invite_resource(created_by, updated_by))
    }

    pub(crate) async fn update_invite(
        &self,
        current_user: RequestUser,
        room_id_or_alias: RoomIdOrAlias,
        invite_code: InviteCode,
        body: PutInviteRequestBody,
    ) -> Result<InviteResource, CaptureApiError> {
        let settings = self.settings_provider.get();

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id_or_alias).await?;
        let tariff = self.get_tariff_for_user(room.created_by).await?;
        verify_invite_write(&tariff, &room)?;

        let RoomInviteWithUsers {
            invite,
            created_by,
            updated_by: _,
        } = inventory
            .get_room_invite_with_creator_and_updater(invite_code)
            .await?;

        if invite.room != room.id {
            return Err(ApiError::not_found().into());
        }

        let now = Timestamp::now();
        let expiration = body.expiration.map(Into::into);
        let invite = inventory
            .update_room_invite(
                room.id,
                invite_code,
                UpdateRoomInvite {
                    updated_by: Some(current_user.id),
                    updated_at: Some(now),
                    expiration: Some(expiration),
                    active: None,
                    room: None,
                },
            )
            .await?;

        // Overwrite the invite in the authorization database
        self.authorizer
            .apply_change(&AuthorizationChange::AddInviteCodeToRoom {
                room: room.id,
                invite_code,
                expiration,
            })
            .await?;

        let created_by = created_by.to_public_user_profile(&settings);
        let updated_by = current_user.to_public_user_profile(&settings);

        Ok(invite.into_invite_resource(created_by, updated_by))
    }

    pub(crate) async fn delete_invite(
        &self,
        current_user: RequestUser,
        room_id_or_alias: RoomIdOrAlias,
        invite_code: InviteCode,
    ) -> Result<(), CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id_or_alias).await?;
        let tariff = self.get_tariff_for_user(room.created_by).await?;
        verify_invite_write(&tariff, &room)?;

        _ = inventory
            .update_room_invite(
                room.id,
                invite_code,
                UpdateRoomInvite {
                    updated_by: Some(current_user.id),
                    updated_at: Some(Timestamp::now()),
                    expiration: None,
                    active: Some(false),
                    room: None,
                },
            )
            .await?;

        self.authorizer
            .apply_change(&AuthorizationChange::RemoveInviteCodeFromRoom {
                room: room.id,
                invite_code,
            })
            .await
            .map_err(|e| {
                log::error!("Could not apply changes in the authorization database: {e:?}");
                ApiError::internal()
            })?;

        Ok(())
    }

    pub(crate) async fn verify_invite_code(
        &self,
        data: PostInviteVerifyRequestBody,
    ) -> Result<PostInviteVerifyResponseBody, CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let invite = inventory.get_room_invite(data.invite_code).await?;
        let room = inventory.get_room(invite.room.into()).await?;
        let tariff = self.get_room_tariff(room.id.into()).await?;

        if !is_invite_valid(&invite, &room, &tariff) {
            // Do not leak the existence of the invite
            return Err(ApiError::not_found().into());
        }

        Ok(PostInviteVerifyResponseBody {
            room_id: invite.room,
            password_required: room.password.is_some(),
        })
    }
}

trait IntoInviteResource {
    fn into_invite_resource(
        self,
        created_by: PublicUserProfile,
        updated_by: PublicUserProfile,
    ) -> InviteResource;
}

impl IntoInviteResource for RoomInvite {
    fn into_invite_resource(
        self,
        created_by: PublicUserProfile,
        updated_by: PublicUserProfile,
    ) -> InviteResource {
        InviteResource {
            invite_code: self.invite_code,
            created: self.created_at.into(),
            created_by,
            updated: self.updated_at.into(),
            updated_by,
            room_id: self.room,
            active: self.active,
            expiration: self.expiration.map(Into::into),
        }
    }
}
