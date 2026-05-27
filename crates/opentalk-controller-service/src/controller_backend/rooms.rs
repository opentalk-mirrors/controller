// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides room-related implementation

use opentalk_controller_api_authorization::authorization::AuthorizationChange;
use opentalk_controller_service_facade::{RequestUser, StartRoomError};
use opentalk_controller_utils::{
    CaptureApiError, TariffResourceExt as _,
    deletion::{Deleter, RoomDeleter},
};
use opentalk_inventory::{
    NewRoom, NewRoomSipConfig, Room, UpdateRoom,
    utils::{build_event_info, is_invite_valid},
};
use opentalk_types_api_v1::{
    error::ApiError,
    pagination::PagePaginationQuery,
    rooms::{GetRoomsResponseBody, RoomResource, by_room_id::GetRoomEventResponseBody},
};
use opentalk_types_common::{
    features::{CALL_IN_MODULE_FEATURE_ID, GUESTS_ALLOWED_MODULE_FEATURE_ID},
    pagination::ItemCount,
    rooms::{GuestAccess, RoomId, RoomPassword, invite_codes::InviteCode},
    users::UserId,
};

use crate::{ControllerBackend, ToUserProfile};

impl ControllerBackend {
    pub(crate) async fn get_rooms(
        &self,
        current_user_id: UserId,
        pagination: &PagePaginationQuery,
    ) -> Result<(GetRoomsResponseBody, ItemCount), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (rooms, room_count) = inventory
            .get_rooms_accessible_to_user_with_creator_paginated(
                current_user_id,
                pagination.per_page,
                pagination.page,
            )
            .await?;

        let rooms = rooms
            .into_iter()
            .map(|(room, user)| RoomResource {
                id: room.id,
                created_by: user.to_public_user_profile(&settings),
                created_at: room.created_at,
                password: room.password,
                waiting_room: room.waiting_room,
                guest_access: room.guest_access,
            })
            .collect::<Vec<RoomResource>>();

        Ok((GetRoomsResponseBody(rooms), room_count))
    }

    pub(crate) async fn create_room(
        &self,
        current_user: RequestUser,
        password: Option<RoomPassword>,
        enable_sip: bool,
        waiting_room: bool,
        guest_access: Option<GuestAccess>,
        e2e_encryption: bool,
    ) -> Result<RoomResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let tariff = self.get_tariff_for_user(current_user.id).await?;

        if enable_sip {
            tariff.require_feature(&CALL_IN_MODULE_FEATURE_ID)?;
        }

        let guest_access = guest_access.unwrap_or(GuestAccess::WaitingRoom);
        if guest_access != GuestAccess::Disabled {
            tariff.require_feature(&GUESTS_ALLOWED_MODULE_FEATURE_ID)?;
        }

        let room = inventory
            .create_room(NewRoom {
                created_by: current_user.id,
                password,
                waiting_room,
                guest_access,
                e2e_encryption,
                tenant_id: current_user.tenant_id,
            })
            .await?;

        if enable_sip {
            _ = inventory
                .create_room_sip_config(NewRoomSipConfig::new(room.id, false))
                .await?;
        }

        drop(inventory);

        let room_resource = RoomResource {
            id: room.id,
            created_by: current_user.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            waiting_room: room.waiting_room,
            guest_access: room.guest_access,
        };

        let is_guest_feature_enabled = tariff.has_feature_enabled(
            &GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
            &GUESTS_ALLOWED_MODULE_FEATURE_ID.feature,
        );
        self.authorizer
            .apply_change(&AuthorizationChange::CreateRoom {
                room: room_resource.id,
                creator: current_user.id,
                is_guest_feature_enabled,
                guest_access: room_resource.guest_access,
                e2e_encryption,
            })
            .await
            .map_err(|e| {
                log::error!("Could not apply changes in the authorization database: {e:?}");
                ApiError::internal()
            })?;

        Ok(room_resource)
    }

    pub(crate) async fn patch_room(
        &self,
        current_user: RequestUser,
        room_id: RoomId,
        password: Option<Option<RoomPassword>>,
        waiting_room: Option<bool>,
        guest_access: Option<GuestAccess>,
        e2e_encryption: Option<bool>,
    ) -> Result<RoomResource, CaptureApiError> {
        let room = self
            .update_room(
                current_user.id,
                room_id,
                password,
                waiting_room,
                guest_access,
                e2e_encryption,
            )
            .await?;

        let settings = self.settings_provider.get();
        let room_resource = RoomResource {
            id: room.id,
            created_by: current_user.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            waiting_room: room.waiting_room,
            guest_access: room.guest_access,
        };

        Ok(room_resource)
    }

    /// Updates a room in the database and applies the necessary changes in the authorization middleware.
    pub(crate) async fn update_room(
        &self,
        created_by: UserId,
        room_id: RoomId,
        password: Option<Option<RoomPassword>>,
        waiting_room: Option<bool>,
        guest_access: Option<GuestAccess>,
        e2e_encryption: Option<bool>,
    ) -> Result<Room, CaptureApiError> {
        let tariff = self.get_tariff_for_user(created_by).await?;

        if guest_access != Some(GuestAccess::Disabled) {
            tariff.require_feature(&GUESTS_ALLOWED_MODULE_FEATURE_ID)?;
        }

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let room = inventory
            .update_room(
                room_id,
                UpdateRoom {
                    password,
                    waiting_room,
                    guest_access,
                    e2e_encryption,
                },
            )
            .await?;

        self.authorizer
            .apply_change(&AuthorizationChange::UpdateRoomConfiguration {
                room: room_id,
                guest_access,
                e2e_encryption,
            })
            .await?;

        Ok(room)
    }

    pub(crate) async fn delete_room(
        &self,
        current_user: RequestUser,
        room_id: RoomId,
        force_delete_reference_if_external_services_fail: bool,
    ) -> Result<(), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let deleter = RoomDeleter::new(room_id, force_delete_reference_if_external_services_fail);

        deleter
            .perform(
                log::logger(),
                inventory.as_mut(),
                self.authorizer.clone(),
                Some(current_user.id),
                &settings,
                &self.storage,
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn get_room(&self, room_id: &RoomId) -> Result<RoomResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (room, created_by) = inventory.get_room_with_creator(*room_id).await?;

        let room_resource = RoomResource {
            id: room.id,
            created_by: created_by.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            waiting_room: room.waiting_room,
            guest_access: room.guest_access,
        };

        Ok(room_resource)
    }

    pub(crate) async fn get_room_event(
        &self,
        room_id: &RoomId,
    ) -> Result<GetRoomEventResponseBody, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let event = inventory.get_event_for_room(*room_id).await?;

        let room = inventory.get_room(*room_id).await?;

        // Naive check that prevents joining events that were created by a user which is since been
        // disabled. The `get_user` method returns 404 not found when the user is disabled.
        if inventory.get_user(room.created_by).await.is_err() {
            return Err(ApiError::forbidden().into());
        }

        let tariff = self.get_tariff_for_user(room.created_by).await?;
        match event.as_ref() {
            Some(event) => {
                let call_in_tel = settings.call_in.as_ref().map(|call_in| call_in.tel.clone());

                let event_info =
                    build_event_info(inventory.as_mut(), call_in_tel, &room, event, &tariff)
                        .await?;

                Ok(GetRoomEventResponseBody(event_info))
            }
            None => Err(ApiError::not_found().into()),
        }
    }

    /// Check the provided invite code and room password
    ///
    /// Returns the associated room
    pub(crate) async fn authenticate_guest(
        &self,
        room_id: RoomId,
        invite_code: Option<InviteCode>,
        password: Option<RoomPassword>,
    ) -> Result<Room, CaptureApiError> {
        let Some(invite_code) = invite_code else {
            return Err(ApiError::not_found().into());
        };

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let (room, created_by) = inventory.get_room_with_creator(room_id).await?;
        let tariff = self.get_tariff_for_user(created_by.id).await?;
        let invite = inventory.get_room_invite(invite_code).await?;

        if !is_invite_valid(&invite, &room, &tariff) {
            // Don't leak the existence of the room
            return Err(ApiError::not_found().into());
        }

        drop(inventory);

        match (&room.password, &password) {
            (Some(_), None) => Err(StartRoomError::WrongRoomPassword.into()),
            (Some(room_password), Some(password)) if password != room_password => {
                Err(StartRoomError::WrongRoomPassword.into())
            }
            _ => Ok(room),
        }
    }
}
