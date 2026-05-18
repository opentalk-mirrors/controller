// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileCopyrightText: OpenTalk Team <mail@opentalk.eu>

use std::sync::Arc;

use opentalk_controller_settings::Settings;
use opentalk_inventory::Inventory;
use opentalk_roomserver_client::{Client, Error, PatchRoomError, RequestTokenError};
use opentalk_roomserver_types::{
    api::RoomServerAccess, client_parameters::ClientParameters,
    room_parameters_patch::RoomParametersPatch,
};
use opentalk_types_api_v1::{error::ApiError, rooms::RoomResource};
use opentalk_types_common::rooms::RoomId;
use url::Url;

use super::build_room_parameters;
use crate::controller_backend::roomserver::RoomServerBackend;

/// A roomserver backend that forwards token requests to an external roomserver.
#[derive(Debug)]
pub(crate) struct ExternalRoomServer {
    client: Client,
}

impl ExternalRoomServer {
    /// Create a new external roomserver backend
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl RoomServerBackend for ExternalRoomServer {
    #[tracing::instrument(level = "debug", skip(self, inventory, settings, room), fields(%room_id = room.id))]
    async fn request_access(
        &self,
        inventory: &mut dyn Inventory,
        settings: Arc<Settings>,
        room: RoomResource,
        client_parameters: ClientParameters,
        _host: Url,
    ) -> Result<RoomServerAccess, ApiError> {
        let room_id = room.id;

        match self
            .client
            .request_token(room_id, client_parameters.clone(), None)
            .await
        {
            Ok(access) => Ok(access),
            Err(Error::ApiError(opentalk_roomserver_client::ApiError {
                code: RequestTokenError::RoomParametersMissing,
                ..
            })) => {
                // The room is unknown to the roomserver — resubmit the token request but include the room parameter
                let room_parameters = build_room_parameters(inventory, settings, room).await?;

                let access = self
                    .client
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
            Err(
                err @ (Error::TokenError(_)
                | Error::UrlParse(_)
                | Error::Reqwest(_)
                | Error::Unexpected { .. }
                | Error::ApiError(opentalk_roomserver_client::ApiError {
                    code:
                        RequestTokenError::InternalServerError | RequestTokenError::InvalidApiToken,
                    ..
                })),
            ) => {
                log::error!("failed to request token from roomserver: {err}");
                Err(ApiError::internal().with_message("failed to request token from roomserver"))
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn patch_room_parameters(
        &self,
        room_id: RoomId,
        patch: RoomParametersPatch,
    ) -> Result<(), ApiError> {
        match self.client.patch_room(room_id, patch).await {
            Ok(()) => Ok(()),
            // The roomserver returns a 404 NotFound when the room does not exist there yet.
            // In this case there is nothing to do and the room parameters will be applied when the room is created.
            Err(Error::ApiError(opentalk_roomserver_client::ApiError {
                code: PatchRoomError::NotFound,
                ..
            })) => Ok(()),
            Err(
                err @ (Error::TokenError(_)
                | Error::UrlParse(_)
                | Error::Reqwest(_)
                | Error::Unexpected { .. }
                | Error::ApiError(opentalk_roomserver_client::ApiError {
                    code: PatchRoomError::InvalidApiToken,
                    ..
                })),
            ) => {
                tracing::error!("Failed to patch roomserver room parameters: {err}");
                Err(ApiError::internal().with_message("Failed to patch roomserver room parameters"))
            }
        }
    }
}
