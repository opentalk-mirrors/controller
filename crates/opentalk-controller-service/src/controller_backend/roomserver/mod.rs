// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides roomserver-related implementation

use std::{collections::BTreeMap, sync::Arc};

use external::ExternalRoomServer;
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::{RoomServerKind, Settings, common::HttpCorsAllowedOrigin};
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Event, Inventory};
use opentalk_roomserver_client::Client;
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
    events::invites::InviteRole,
    rooms::RoomId,
    shared_folders::{SharedFolder, SharedFolderAccess},
    tariffs::QuotaType,
    users::{UserId, UserInfo},
};
use tokio::sync::broadcast::Receiver;

use crate::{
    ControllerBackend, controller_backend::roomserver::internal::InternalRoomServer,
    email_to_libravatar_url, helpers::get_user_timezone,
};

mod external;
mod internal;
mod storage_notifier;
mod websocket_adapter;

pub use storage_notifier::build_storage_notifier;

/// Creates a RoomServer instance
pub fn build_roomserver(
    kind: &RoomServerKind,
    shutdown: Receiver<()>,
) -> Arc<dyn RoomServerBackend + Send + Sync> {
    match kind {
        RoomServerKind::Internal {
            settings,
            public_url,
        } => {
            log::debug!("Using internal roomserver");
            Arc::new(InternalRoomServer::new(
                settings.clone(),
                public_url.clone(),
                shutdown,
            ))
        }
        RoomServerKind::External {
            service_url,
            api_key,
        } => {
            log::debug!("Using external roomserver");
            let roomserver_client = Client::new(service_url.clone(), api_key.clone());
            Arc::new(ExternalRoomServer::new(roomserver_client))
        }
    }
}

/// A trait for roomserver backends that can be used by the controller.
#[async_trait::async_trait]
pub trait RoomServerBackend {
    /// Request a room access token from the roomserver.
    async fn request_access(
        &self,
        inventory: &mut dyn Inventory,
        settings: Arc<Settings>,
        room: RoomResource,
        client_parameters: ClientParameters,
    ) -> Result<RoomServerAccess, ApiError>;

    /// Patch the parameters of a room
    ///
    /// This function does nothing and returns no error when the specified room does not exist.
    async fn patch_room_parameters(
        &self,
        room_id: RoomId,
        patch: RoomParametersPatch,
    ) -> Result<(), ApiError>;
}

impl ControllerBackend {
    #[tracing::instrument(level = "debug", skip(self, user, request), fields(user_id = %user.id))]
    pub(crate) async fn start_room(
        &self,
        user: RequestUser,
        room_id: RoomId,
        request: PostRoomsRoomserverStartRequestBody,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;
        let settings = self.settings_provider.get();

        let room = self.get_room(&room_id).await?;
        let role =
            Self::get_user_role(inventory.as_mut(), user.id, room_id, room.created_by.id).await?;
        let timezone = get_user_timezone(room.created_by.id, inventory.as_mut(), &settings).await;
        let avatar_url = user.avatar_url.unwrap_or_else(|| {
            email_to_libravatar_url(&settings.avatar.libravatar_url, &user.email)
        });

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
                        avatar_url,
                    },
                    timezone,
                },
            },
            role,
        };

        let access = self
            .roomserver
            .request_access(inventory.as_mut(), settings, room, client_parameters)
            .await?;

        Ok(RoomserverStartResponseBody {
            token: access.token,
            roomserver_address: access.public_url.to_string(),
        })
    }

    #[tracing::instrument(level = "debug", skip(self, request))]
    pub(crate) async fn start_room_invited(
        &self,
        room_id: RoomId,
        request: PostRoomsRoomserverStartInvitedRequestBody,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
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

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let settings = self.settings_provider.get();
        let access = self
            .roomserver
            .request_access(
                inventory.as_mut(),
                settings,
                room_resource,
                client_parameters,
            )
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

        self.roomserver.patch_room_parameters(room_id, patch).await
    }

    async fn get_user_role(
        inventory: &mut dyn Inventory,
        user_id: UserId,
        room_id: RoomId,
        room_creator: UserId,
    ) -> Result<Role, CaptureApiError> {
        if user_id == room_creator {
            return Ok(Role::Moderator);
        }

        let role = match inventory
            .get_event_invite_for_user_and_room(user_id, room_id)
            .await?
            .map(|invite| invite.role)
        {
            Some(InviteRole::Moderator) => Role::Moderator,
            None | Some(InviteRole::User) => Role::User,
        };

        Ok(role)
    }
}

pub(crate) async fn build_room_parameters(
    inventory: &mut dyn Inventory,
    settings: Arc<Settings>,
    room_resource: RoomResource,
) -> Result<RoomParameters, CaptureApiError> {
    let call_in = get_call_in_info(inventory, &settings, room_resource.id).await?;
    let room = inventory.get_room(room_resource.id).await?;

    let db_event = inventory.get_event_for_room(room_resource.id).await?;
    let show_meeting_details = db_event
        .as_ref()
        .map(|event| event.show_meeting_details)
        .unwrap_or(false);
    let event = match db_event {
        Some(event) => build_event_context(event, inventory).await?,
        None => None,
    };

    let streaming_targets = inventory
        .get_room_streaming_targets(room_resource.id)
        .await?;

    let invite_code = inventory
        .get_valid_invite_for_room(room_resource.id)
        .await?
        .map(|invite| invite.invite_code);

    let tariff = inventory
        .get_tariff_for_user(room_resource.created_by.id)
        .await?;

    let mut module_settings = settings.roomserver.modules.clone();
    override_module_settings(inventory, room_resource.id, &mut module_settings).await?;

    let disabled_modules = tariff.disabled_modules();
    module_settings.retain(|module_id, _| !disabled_modules.contains(module_id));

    let mut used_quota = BTreeMap::new();
    for quota_type in tariff.quotas.keys() {
        let used = match quota_type {
            QuotaType::RoomParticipantLimit | QuotaType::RoomTimeLimitSecs => 0,
            QuotaType::MaxStorage => inventory
                .get_user_storage_used_size_u64(room_resource.created_by.id)
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

    let user_id = room_resource.created_by.id;
    let timezone = get_user_timezone(user_id, inventory, &settings).await;
    let created_by = PublicUserProfile {
        id: room_resource.created_by.id,
        email: room_resource.created_by.email,
        user_info: room_resource.created_by.user_info,
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
        password: room_resource.password,
        waiting_room: room_resource.waiting_room,
        call_in,
        event,
        invite_code,
        tariff,
        streaming_targets,
        show_meeting_details,
        e2e_encryption: room.e2e_encryption,
        module_settings,
        preferred_language,
        fallback_language: settings.defaults.user_language.clone(),
        ws_rate_limit: settings.roomserver.websocket_rate_limit,
        allowed_origins,
        room_idle_timeout: settings.roomserver.room_idle_timeout,
    };

    Ok(parameters)
}

pub(crate) async fn get_call_in_info(
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

pub(crate) async fn build_event_context(
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
pub(crate) async fn override_module_settings(
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
