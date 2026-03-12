// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides roomserver-related implementation

use std::collections::BTreeMap;

use opentalk_controller_service_facade::{RequestUser, StartRoomError};
use opentalk_controller_settings::{Settings, common::HttpCorsAllowedOrigin};
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Event, Inventory};
use opentalk_roomserver_client::{Error, PatchRoomError, RequestTokenError};
use opentalk_roomserver_types::{
    api::RoomServerAccess,
    client_parameters::{ClientKind, ClientParameters, Role},
    module_settings::ModuleSettings,
    public_user_profile::PublicUserProfile,
    room_parameters::{EventContext, RoomParameters},
    room_parameters_patch::RoomParametersPatch,
    tariff_details::TariffDetails,
};
use opentalk_roomserver_types_training_participation_report::settings::TrainingParticipationReportSettings;
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::{
        RoomResource,
        by_room_id::{
            PostRoomsRoomserverStartInvitedRequestBody, PostRoomsRoomserverStartRequestBody,
            RoomserverStartResponseBody,
        },
    },
};
use opentalk_types_common::{
    call_in::CallInInfo,
    rooms::RoomId,
    shared_folders::{SharedFolder, SharedFolderAccess},
    tariffs::QuotaType,
    users::UserInfo,
};

use crate::{ControllerBackend, helpers::get_user_timezone};

impl ControllerBackend {
    #[tracing::instrument(level = "debug", skip(self, user, request), fields(user_id = %user.id))]
    pub(crate) async fn roomserver_start_room(
        &self,
        user: RequestUser,
        room_id: RoomId,
        request: PostRoomsRoomserverStartRequestBody,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
        if self.roomserver_client.is_none() {
            return Err(StartRoomError::RoomserverSignalingDisabled.into());
        };

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let settings = self.settings_provider.get();

        let room = self.get_room(&room_id).await?;

        let role = if room.created_by.id == user.id {
            Role::Moderator
        } else {
            Role::User
        };

        let timezone = get_user_timezone(room.created_by.id, inventory.as_mut(), &settings).await;

        let client_parameters = ClientParameters {
            device_secret: request.device_secret,
            kind: ClientKind::Registered {
                profile: PublicUserProfile {
                    id: user.id,
                    email: user.email,
                    user_info: UserInfo {
                        title: user.title,
                        firstname: user.firstname,
                        lastname: user.lastname,
                        display_name: request.display_name.unwrap_or(user.display_name),
                        avatar_url: user
                            .avatar_url
                            .unwrap_or(settings.avatar.libravatar_url.clone()),
                    },
                    timezone,
                },
            },
            role,
        };

        let access = self
            .request_roomserver_access(room, client_parameters)
            .await?;

        Ok(RoomserverStartResponseBody {
            token: access.token,
            roomserver_address: access.public_url.to_string(),
        })
    }

    #[tracing::instrument(level = "debug", skip(self, request))]
    pub(crate) async fn roomserver_start_room_invited(
        &self,
        room_id: RoomId,
        request: PostRoomsRoomserverStartInvitedRequestBody,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
        if self.roomserver_client.is_none() {
            return Err(StartRoomError::RoomserverSignalingDisabled.into());
        };

        let _ = self
            .authenticate_guest(&room_id, &request.invite_code, &request.password)
            .await?;

        let room_resource = self.get_room(&room_id).await?;

        let client_parameters = ClientParameters {
            device_secret: request.device_secret,
            kind: ClientKind::Guest {
                display_name: request.display_name,
            },
            role: Role::User,
        };

        let access = self
            .request_roomserver_access(room_resource, client_parameters)
            .await?;

        Ok(RoomserverStartResponseBody {
            token: access.token,
            roomserver_address: access.public_url.to_string(),
        })
    }

    pub(crate) async fn patch_room_parameters(
        &self,
        room_id: RoomId,
        patch: RoomParametersPatch,
    ) -> Result<(), ApiError> {
        if patch.is_empty() {
            // No changes to apply
            return Ok(());
        }

        let Some(client) = &self.roomserver_client else {
            // When the roomserver is not configured, there is nothing to do.
            return Ok(());
        };

        match client.patch_room(room_id, patch).await {
            Ok(()) => Ok(()),
            // The roomserver returns a 404 NotFound when the room does not exist there yet.
            // In this case there is nothing to do and the room parameters will be applied when the room is created.
            Err(Error::ApiError(opentalk_roomserver_client::ApiError {
                code: PatchRoomError::NotFound,
                ..
            })) => Ok(()),
            Err(err) => {
                tracing::error!("Failed to patch roomserver room parameters: {err}");
                Err(ApiError::internal().with_message("Failed to patch roomserver room parameters"))
            }
        }
    }

    pub(crate) async fn request_roomserver_access(
        &self,
        room: RoomResource,
        client_parameters: ClientParameters,
    ) -> Result<RoomServerAccess, ApiError> {
        let room_id = room.id;

        let Some(client) = &self.roomserver_client else {
            return Err(ApiError::internal()
                .with_message("roomserver is not configured on this controller"));
        };

        match client
            .request_token(room_id, client_parameters.clone(), None)
            .await
        {
            Ok(access) => Ok(access),
            Err(Error::ApiError(opentalk_roomserver_client::ApiError {
                code: RequestTokenError::RoomParametersMissing,
                ..
            })) => {
                // The room is unknown to the roomserver,- resubmit the token request but include the room parameter
                let room_parameters = self.build_room_parameters(room).await?;

                let access = client
                    .request_token(room_id, client_parameters, Some(room_parameters))
                    .await
                    .map_err(|e| {
                        log::error!("failed to request token from roomserver: {e:?}");

                        ApiError::internal().with_message("failed to request token from roomserver")
                    })?;

                Ok(access)
            }
            Err(Error::ApiError(opentalk_roomserver_client::ApiError {
                code: RequestTokenError::Banned,
                ..
            })) => {
                log::debug!("attempted to request a token for a banned user");
                Err(ApiError::forbidden().with_message("you are banned from this room"))
            }
            Err(err) => {
                log::error!("failed to request token from roomserver: {err}");
                Err(ApiError::internal().with_message("failed to request token from roomserver"))
            }
        }
    }

    async fn build_room_parameters(
        &self,
        room: RoomResource,
    ) -> Result<RoomParameters, CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let settings = self.settings_provider.get();
        let room_server_settings = settings.roomserver.as_ref().ok_or_else(|| {
            ApiError::internal().with_message("roomserver settings are not configured")
        })?;

        let call_in = Self::get_call_in_info(inventory.as_mut(), &settings, room.id).await?;

        let db_event = inventory.get_event_for_room(room.id).await?;
        let show_meeting_details = db_event
            .as_ref()
            .map(|event| event.show_meeting_details)
            .unwrap_or(false);
        let event = match db_event {
            Some(event) => Self::build_event_context(event, inventory.as_mut()).await?,
            None => None,
        };

        let streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

        let invite_code = inventory
            .get_valid_invite_for_room(room.id)
            .await?
            .map(|invite| invite.invite_code);

        let tariff = inventory.get_tariff_for_user(room.created_by.id).await?;

        let mut module_settings = room_server_settings.modules.clone();
        Self::override_module_settings(inventory.as_mut(), room.id, &mut module_settings).await?;

        let disabled_modules = tariff.disabled_modules();
        module_settings.retain(|module_id, _| !disabled_modules.contains(module_id));

        let mut used_quota = BTreeMap::new();
        for quota_type in tariff.quotas.keys() {
            let used = match quota_type {
                QuotaType::RoomParticipantLimit | QuotaType::RoomTimeLimitSecs => 0,
                QuotaType::MaxStorage => inventory
                    .get_user_storage_used_size_u64(room.created_by.id)
                    .await
                    .unwrap_or(0),
            };
            // This can't be None because the quota_type are the keys in the quotas BTreeMap
            let _ = used_quota.insert(quota_type.clone(), used);
        }

        let disabled_features = tariff
            .disabled_features()
            .into_iter()
            .chain(settings.defaults.disabled_features.iter().cloned())
            .collect();
        let tariff = TariffDetails {
            id: tariff.id,
            name: tariff.name,
            quotas: tariff.quotas,
            used_quota,
            disabled_features,
        };

        let user_id = room.created_by.id;
        let timezone = get_user_timezone(user_id, inventory.as_mut(), &settings).await;
        let created_by = PublicUserProfile {
            id: room.created_by.id,
            email: room.created_by.email,
            user_info: room.created_by.user_info,
            timezone,
        };
        let preferred_language = inventory
            .get_user(user_id)
            .await
            .ok()
            .and_then(|user| user.language)
            .map(|language| language.0)
            .unwrap_or(settings.defaults.user_language.clone());

        let allowed_origins = settings
            .http
            .cors
            .allowed_origin
            .as_ref()
            .map(|origins| {
                origins
                    .iter()
                    .map(HttpCorsAllowedOrigin::header_value)
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    settings
                        .frontend
                        .base_url
                        .to_string()
                        .trim_end_matches('/')
                        .to_string(),
                ]
            });

        let parameters = RoomParameters {
            created_by,
            password: room.password,
            waiting_room: room.waiting_room,
            call_in,
            event,
            invite_code,
            tariff,
            streaming_targets,
            show_meeting_details,
            e2e_encryption: false,
            module_settings,
            preferred_language,
            fallback_language: settings.defaults.user_language.clone(),
            ws_rate_limit: room_server_settings.websocket_rate_limit,
            allowed_origins,
        };

        Ok(parameters)
    }

    async fn build_event_context(
        event: Event,
        inventory: &mut dyn Inventory,
    ) -> Result<Option<EventContext>, CaptureApiError> {
        let shared_folder = match inventory.get_event_shared_folder(event.id).await? {
            Some(event_shared_folder) => Some(SharedFolder {
                read: SharedFolderAccess {
                    url: event_shared_folder.read_url,
                    password: event_shared_folder.read_password,
                },
                read_write: Some(SharedFolderAccess {
                    url: event_shared_folder.write_url,
                    password: event_shared_folder.write_password,
                }),
            }),
            None => None,
        };

        let context = EventContext {
            id: event.id,
            starts_at: event.starts_at().map(Into::into),
            ends_at: event.ends_at().map(Into::into),
            is_adhoc: event.is_adhoc,
            shared_folder,
            // Note: Title and description do not impl Copy, so they partially
            // move event.
            title: event.title,
            description: event.description,
        };

        Ok(Some(context))
    }

    /// Overrides module settings with values from the inventory.
    async fn override_module_settings(
        inventory: &mut dyn Inventory,
        room_id: RoomId,
        module_settings: &mut ModuleSettings,
    ) -> Result<(), CaptureApiError> {
        let Some(event) = inventory.get_event_for_room(room_id).await? else {
            return Ok(());
        };

        if let Some(autostart) = inventory
            .get_event_training_participation_report_parameter_set(event.id)
            .await?
        {
            let settings = TrainingParticipationReportSettings {
                autostart: Some(autostart.into()),
            };
            module_settings.insert(&settings).map_err(|err| {
                log::error!("failed to serialize training participation report module settings for room {room_id}: {err}");
                ApiError::internal()
            })?;
        }

        Ok(())
    }

    async fn get_call_in_info(
        inventory: &mut dyn Inventory,
        settings: &Settings,
        room_id: RoomId,
    ) -> Result<Option<CallInInfo>, CaptureApiError> {
        let Some(tel) = settings.call_in.as_ref().map(|call_in| call_in.tel.clone()) else {
            return Ok(None);
        };

        Ok(inventory
            .get_room_sip_config(room_id)
            .await?
            .map(|sip_config| CallInInfo {
                tel,
                id: sip_config.sip_id,
                password: sip_config.password,
            }))
    }
}
