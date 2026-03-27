// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides room-related implementation

use std::str::FromStr;

use kustos::{
    AccessMethod, Resource,
    policies_builder::{GrantingAccess, PoliciesBuilder},
    prelude::IsSubject,
};
use opentalk_controller_service_facade::{RequestUser, StartRoomError};
use opentalk_controller_utils::{
    CaptureApiError, TariffResourceExt as _,
    deletion::{Deleter, RoomDeleter},
};
use opentalk_inventory::{NewRoom, NewRoomSipConfig, Room, UpdateRoom, utils::build_event_info};
use opentalk_signaling_core::Participant;
use opentalk_signaling_module_breakout::BreakoutStorageProvider as _;
use opentalk_signaling_module_moderation::ModerationStorageProvider as _;
use opentalk_types_api_v1::{
    error::{ApiError, ERROR_CODE_INVALID_VALUE, ValidationErrorEntry},
    pagination::PagePaginationQuery,
    rooms::{
        GetRoomsResponseBody, RoomResource,
        by_room_id::{
            GetRoomEventResponseBody, PostRoomsStartInvitedRequestBody, PostRoomsStartRequestBody,
            RoomsStartResponseBody,
        },
    },
};
use opentalk_types_common::{
    features::{self, GUESTS_ALLOWED_FEATURE_ID},
    modules::CORE_MODULE_ID,
    pagination::ItemCount,
    rooms::{RoomId, RoomPassword, invite_codes::InviteCode},
    tariffs::TariffResource,
    users::UserId,
};

use crate::{
    ControllerBackend, ToUserProfile, signaling::ticket::start_or_continue_signaling_session,
};

pub mod roomserver;

impl ControllerBackend {
    pub(crate) async fn get_rooms(
        &self,
        current_user_id: UserId,
        pagination: &PagePaginationQuery,
    ) -> Result<(GetRoomsResponseBody, ItemCount), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let accessible_rooms: kustos::AccessibleResources<RoomId> = self
            .authz
            .get_accessible_resources_for_user(current_user_id, AccessMethod::Get)
            .await?;

        let (rooms, room_count) = match accessible_rooms {
            kustos::AccessibleResources::All => {
                inventory
                    .get_all_rooms_paginated_with_creator(pagination.per_page, pagination.page)
                    .await?
            }
            kustos::AccessibleResources::List(list) => {
                inventory
                    .get_rooms_paginated_by_id_with_creator(
                        &list,
                        pagination.per_page,
                        pagination.page,
                    )
                    .await?
            }
        };

        let rooms = rooms
            .into_iter()
            .map(|(room, user)| RoomResource {
                id: room.id,
                created_by: user.to_public_user_profile(&settings),
                created_at: room.created_at,
                password: room.password,
                waiting_room: room.waiting_room,
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
        e2e_encryption: bool,
    ) -> Result<RoomResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let tariff = self.get_tariff_for_user(current_user.id).await?;

        if enable_sip {
            tariff.require_feature(&features::CALL_IN_MODULE_FEATURE_ID)?;
        }

        let room = inventory
            .create_room(NewRoom {
                created_by: current_user.id,
                password,
                waiting_room,
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
        };

        let policies = PoliciesBuilder::new()
            .grant_user_access(current_user.id)
            .room_read_access(room_resource.id)
            .room_write_access(room_resource.id)
            .finish();

        self.authz.add_policies(policies).await?;

        Ok(room_resource)
    }

    pub(crate) async fn patch_room(
        &self,
        current_user: RequestUser,
        room_id: RoomId,
        password: Option<Option<RoomPassword>>,
        waiting_room: Option<bool>,
        e2e_encryption: Option<bool>,
    ) -> Result<RoomResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory
            .update_room(
                room_id,
                UpdateRoom {
                    password,
                    waiting_room,
                    e2e_encryption,
                },
            )
            .await?;

        let room_resource = RoomResource {
            id: room.id,
            created_by: current_user.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            waiting_room: room.waiting_room,
        };

        Ok(room_resource)
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
                &self.authz,
                Some(current_user.id),
                self.exchange_handle.clone(),
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
        };

        Ok(room_resource)
    }

    pub(crate) async fn get_room_tariff(
        &self,
        room_id: RoomId,
        invite_code: Option<InviteCode>,
    ) -> Result<TariffResource, CaptureApiError> {
        let tariff = self.get_tariff_for_room(room_id).await?;

        if invite_code.is_some()
            && !tariff.has_feature_enabled(&CORE_MODULE_ID, &GUESTS_ALLOWED_FEATURE_ID)
        {
            return Err(ApiError::not_found().into());
        }

        Ok(tariff)
    }

    pub(crate) async fn get_room_event(
        &self,
        room_id: &RoomId,
        invite_code: Option<InviteCode>,
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

        if invite_code.is_some()
            && !tariff.has_feature_enabled(&CORE_MODULE_ID, &GUESTS_ALLOWED_FEATURE_ID)
        {
            return Err(ApiError::not_found().into());
        }

        match event.as_ref() {
            Some(event) => {
                let call_in_tel = settings.call_in.as_ref().map(|call_in| call_in.tel.clone());

                let event_info = build_event_info(
                    inventory.as_mut(),
                    call_in_tel,
                    *room_id,
                    room.e2e_encryption,
                    event,
                    &tariff,
                )
                .await?;

                Ok(GetRoomEventResponseBody(event_info))
            }
            None => Err(ApiError::not_found().into()),
        }
    }

    pub(crate) async fn start_room_session(
        &self,
        current_user: RequestUser,
        room_id: RoomId,
        request: PostRoomsStartRequestBody,
    ) -> Result<RoomsStartResponseBody, CaptureApiError> {
        if self.settings_provider.get().roomserver.is_some() {
            return Err(StartRoomError::LegacySignalingDisabled.into());
        }

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let mut volatile = self.volatile.clone();

        let room = inventory.get_room(room_id).await?;

        // check if user is banned from room
        if volatile
            .moderation_storage()
            .is_user_banned(room.id, current_user.id)
            .await
            .map_err(Into::<ApiError>::into)?
        {
            return Err(StartRoomError::BannedFromRoom.into());
        }

        if let Some(breakout_room) = request.breakout_room {
            let config = volatile
                .breakout_storage()
                .get_breakout_config(room.id)
                .await
                .map_err(Into::<ApiError>::into)?;

            if let Some(config) = config {
                if !config.is_valid_id(breakout_room) {
                    return Err(StartRoomError::InvalidBreakoutRoomId.into());
                }
            } else {
                return Err(StartRoomError::NoBreakoutRooms.into());
            }
        }

        let (ticket, resumption) = start_or_continue_signaling_session(
            &mut volatile,
            current_user.id.into(),
            room_id,
            request.breakout_room,
            request.resumption,
        )
        .await?;

        Ok(RoomsStartResponseBody { ticket, resumption })
    }

    pub(crate) async fn start_invited_room_session(
        &self,
        room_id: RoomId,
        request: PostRoomsStartInvitedRequestBody,
    ) -> Result<RoomsStartResponseBody, CaptureApiError> {
        if self.settings_provider.get().roomserver.is_some() {
            return Err(StartRoomError::LegacySignalingDisabled.into());
        }

        let room = self
            .authenticate_guest(&room_id, &request.invite_code, &request.password)
            .await?;

        let mut volatile = self.volatile.clone();

        if let Some(breakout_room) = request.breakout_room {
            let config = volatile
                .breakout_storage()
                .get_breakout_config(room.id)
                .await
                .map_err(Into::<ApiError>::into)?;

            if let Some(config) = config {
                if !config.is_valid_id(breakout_room) {
                    return Err(StartRoomError::InvalidBreakoutRoomId.into());
                }
            } else {
                return Err(StartRoomError::NoBreakoutRooms.into());
            }
        }

        let (ticket, resumption) = start_or_continue_signaling_session(
            &mut volatile,
            Participant::Guest,
            room_id,
            request.breakout_room,
            request.resumption,
        )
        .await?;

        Ok(RoomsStartResponseBody { ticket, resumption })
    }

    /// Check the provided invite code and room password
    ///
    /// Returns the associated room
    pub(crate) async fn authenticate_guest(
        &self,
        room_id: &RoomId,
        invite_code: &str,
        password: &Option<RoomPassword>,
    ) -> Result<Room, CaptureApiError> {
        let invite_code_as_uuid = uuid::Uuid::from_str(invite_code).map_err(|_| {
            ApiError::unprocessable_entities([ValidationErrorEntry::new(
                "invite_code",
                ERROR_CODE_INVALID_VALUE,
                Some("Bad invite code format"),
            )])
        })?;

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let invite = inventory
            .get_room_invite(InviteCode::from(invite_code_as_uuid))
            .await?;

        if !invite.active {
            return Err(ApiError::not_found().into());
        }

        if invite.room != *room_id {
            return Err(ApiError::bad_request()
                .with_message("Room id mismatch")
                .into());
        }

        let room = inventory.get_room(invite.room).await?;

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

/// Provides functionality to grant room privileges
pub trait RoomsPoliciesBuilderExt {
    /// Set the room privileges needed to grant read access to guests
    #[allow(unused)]
    fn room_guest_read_access(self, room_id: RoomId) -> Self;
    /// Set the room privileges needed to grant read access
    fn room_read_access(self, room_id: RoomId) -> Self;
    /// Set the room privileges needed to grant write access
    fn room_write_access(self, room_id: RoomId) -> Self;
}

impl<T> RoomsPoliciesBuilderExt for PoliciesBuilder<GrantingAccess<T>>
where
    T: IsSubject + Clone,
{
    fn room_guest_read_access(self, room_id: RoomId) -> Self {
        self.add_resource(
            room_id.resource_id().with_suffix("/tariff"),
            [AccessMethod::Get],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/event"),
            [AccessMethod::Get],
        )
    }

    fn room_read_access(self, room_id: RoomId) -> Self {
        self.add_resource(room_id.resource_id(), [AccessMethod::Get])
            .add_resource(
                room_id.resource_id().with_suffix("/invites"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/streaming_targets"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/start"),
                [AccessMethod::Post],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/tariff"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/event"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/assets"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/assets/*"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/assets/*/download"),
                [AccessMethod::Get],
            )
            .add_resource(
                room_id.resource_id().with_suffix("/roomserver/start"),
                [AccessMethod::Post],
            )
    }

    fn room_write_access(self, room_id: RoomId) -> Self {
        self.add_resource(
            room_id.resource_id(),
            [AccessMethod::Patch, AccessMethod::Delete],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/invites"),
            [AccessMethod::Post],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/streaming_targets"),
            [AccessMethod::Post],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/invites/*"),
            [AccessMethod::Get, AccessMethod::Put, AccessMethod::Delete],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/streaming_targets/*"),
            [AccessMethod::Get, AccessMethod::Patch, AccessMethod::Delete],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/assets"),
            [AccessMethod::Post, AccessMethod::Delete],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/assets/*"),
            [AccessMethod::Delete],
        )
        .add_resource(
            room_id.resource_id().with_suffix("/roomserver/*"),
            [AccessMethod::Post],
        )
    }
}
