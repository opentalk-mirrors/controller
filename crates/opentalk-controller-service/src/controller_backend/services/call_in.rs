// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::str::FromStr;

use opentalk_controller_utils::{CaptureApiError, TariffResourceExt as _};
use opentalk_inventory::{Room, User};
use opentalk_roomserver_types::client_parameters::{ClientKind, ClientParameters, Role};
use opentalk_types_api_internal::call_in::PostCallInStartRoomServerRequestBody;
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::{RoomResource, by_room_id::RoomserverStartResponseBody},
    services::call_in::PostCallInStartRequestBody,
};
use opentalk_types_common::{
    features,
    roomserver::{DEVICE_SECRET_MIN_LENGTH, DeviceSecret},
};
use rand::RngExt;
use url::Url;

use crate::{ControllerBackend, ToUserProfile};

impl ControllerBackend {
    async fn check_call_in_request(
        &self,
        request: PostCallInStartRequestBody,
    ) -> Result<(Room, User), CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (sip_config, room, creator) = inventory
            .get_room_sip_config_with_room_and_creator(request.id)
            .await?
            .ok_or_else(invalid_credentials_error)?;

        if room.e2e_encryption {
            return Err(ApiError::forbidden()
                .with_code("service_unavailable")
                .with_message("call-in is not available for encrypted rooms")
                .into());
        }

        let tariff = self.get_tariff_for_user(room.created_by).await?;

        tariff.require_feature(&features::CALL_IN_MODULE_FEATURE_ID)?;

        if sip_config.password != request.pin {
            return Err(invalid_credentials_error().into());
        }

        Ok((room, creator))
    }

    pub(crate) async fn start_call_in_roomserver_impl(
        &self,
        request: PostCallInStartRoomServerRequestBody,
        host: Url,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
        let settings = self.settings_provider.get();

        let (room, creator) = self
            .check_call_in_request(PostCallInStartRequestBody {
                id: request.id.clone(),
                pin: request.pin.clone(),
            })
            .await?;

        let room_resource = RoomResource {
            id: room.id,
            created_by: creator.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            guest_access: room.guest_access,
            waiting_room: room.waiting_room,
        };

        let device_secret = rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(DEVICE_SECRET_MIN_LENGTH)
            .map(|c| c as char)
            .collect::<String>();
        let device_secret = DeviceSecret::from_str(&device_secret)
            .expect("String is at least DEVICE_SECRET_MIN_LENGTH long");

        let client_parameters = ClientParameters {
            device_secret,
            kind: ClientKind::CallIn {
                display_name: request.display_name,
            },
            role: Role::User,
        };

        let mut inventory = self.inventory_provider.get_inventory().await?;
        let access = self
            .roomserver
            .request_access(
                inventory.as_mut(),
                settings,
                room_resource,
                client_parameters,
                host,
            )
            .await?;

        Ok(RoomserverStartResponseBody {
            token: access.token,
            roomserver_address: access.public_url.to_string(),
        })
    }
}

fn invalid_credentials_error() -> ApiError {
    ApiError::bad_request()
        .with_code("invalid_credentials")
        .with_message("given call-in id & pin combination is not valid")
}
