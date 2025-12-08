// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides roomserver-related implementation

use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::Settings;
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::Inventory;
use opentalk_roomserver_client::{Error, RequestTokenError};
use opentalk_roomserver_types::{
    api::RoomServerAccess,
    client_parameters::{ClientKind, ClientParameters, Role},
    module_settings::ModuleSettings,
    public_user_profile::PublicUserProfile,
    room_parameters::{EventContext, RoomParameters},
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
    streaming::StreamingLink,
    users::UserInfo,
};

use crate::{
    ControllerBackend, controller_backend::rooms::start_room_error::StartRoomError,
    helpers::get_user_timezone,
};

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

    async fn request_roomserver_access(
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

        let event = Self::get_event_context(inventory.as_mut(), room.id).await?;

        let streaming_links = Self::build_streaming_links(inventory.as_mut(), room.id).await?;

        let invite_code = inventory
            .get_valid_invite_for_room(room.id)
            .await?
            .map(|invite| invite.invite_code);

        let tariff = self.get_tariff_for_room(room.id).await?;

        let mut module_settings = room_server_settings.modules.clone();

        Self::override_module_settings(inventory.as_mut(), room.id, &mut module_settings).await?;
        module_settings.retain(|module_id, _| tariff.modules.contains_key(module_id));

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
            .map(|user| user.language.0)
            .unwrap_or(settings.defaults.user_language.clone());
        let parameters = RoomParameters {
            created_by,
            password: room.password,
            waiting_room: room.waiting_room,
            call_in,
            event,
            invite_code,
            tariff,
            streaming_links,
            e2e_encryption: false,
            module_settings,
            asset_storage: room_server_settings.asset_storage.clone(),
            preferred_language,
            fallback_language: settings.defaults.user_language.clone(),
        };

        Ok(parameters)
    }

    async fn get_event_context(
        inventory: &mut dyn Inventory,
        room_id: RoomId,
    ) -> Result<Option<EventContext>, CaptureApiError> {
        let Some(event) = inventory.get_event_for_room(room_id).await? else {
            return Ok(None);
        };

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
            title: event.title,
            description: event.description,
            is_adhoc: event.is_adhoc,
            starts_at: event.starts_at.map(Into::into),
            ends_at: event.ends_at.map(Into::into),
            shared_folder,
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

    async fn build_streaming_links(
        inventory: &mut dyn Inventory,
        room_id: RoomId,
    ) -> Result<Vec<StreamingLink>, CaptureApiError> {
        let streaming_targets = inventory.get_room_streaming_target_records(room_id).await?;
        let mut streaming_links = Vec::new();

        for target in streaming_targets {
            let url = match target.public_url.parse() {
                Ok(url) => url,
                Err(err) => {
                    log::warn!(
                        "Unparsable streaming url in streaming records for room {room_id}: {err}"
                    );
                    continue;
                }
            };

            streaming_links.push(StreamingLink {
                name: target.name,
                url,
            });
        }

        Ok(streaming_links)
    }
}
