// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_utils::CaptureApiError;
use opentalk_db_storage::sip_configs::{NewSipConfig, UpdateSipConfig};
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::by_room_id::sip::{PutSipConfigRequestBody, SipConfigResource},
};
use opentalk_types_common::{features, rooms::RoomId};

use crate::{ControllerBackend, require_feature};

impl ControllerBackend {
    pub(crate) async fn get_sip_config(
        &self,
        room_id: RoomId,
    ) -> Result<SipConfigResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id).await?;

        if room.e2e_encryption {
            return Err(ApiError::not_found()
                .with_code("service_unavailable")
                .with_message("Call-in not available for end-to-end encrypted room".to_string())
                .into());
        }

        require_feature(
            inventory.as_mut(),
            &settings,
            room.created_by,
            &features::CALL_IN_MODULE_FEATURE_ID,
        )
        .await?;

        let config = inventory
            .get_room_sip_config(room_id)
            .await?
            .ok_or_else(ApiError::not_found)?;

        Ok(SipConfigResource {
            room: room_id,
            sip_id: config.sip_id,
            password: config.password,
            lobby: config.lobby,
        })
    }

    pub(crate) async fn set_sip_config(
        &self,
        room_id: RoomId,
        modify_sip_config: PutSipConfigRequestBody,
    ) -> Result<(SipConfigResource, bool), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id).await?;

        if room.e2e_encryption {
            return Err(ApiError::forbidden()
                .with_code("service_unavailable")
                .with_message("Call-in not available for end-to-end encrypted room".to_string())
                .into());
        }

        require_feature(
            inventory.as_mut(),
            &settings,
            room.created_by,
            &features::CALL_IN_MODULE_FEATURE_ID,
        )
        .await?;

        let changeset = UpdateSipConfig {
            password: modify_sip_config.password.clone(),
            enable_lobby: modify_sip_config.lobby,
        };

        // FIXME: use on_conflict().do_update() (UPSERT) for this PUT
        // Try to modify the sip config before creating a new one
        let (sip_config, newly_created) = if let Some(db_sip_config) =
            inventory.update_room_sip_config(room_id, changeset).await?
        {
            let sip_config = SipConfigResource {
                room: room_id,
                sip_id: db_sip_config.sip_id,
                password: db_sip_config.password,
                lobby: db_sip_config.lobby,
            };

            (sip_config, false)
        } else {
            // Create a new sip config
            let mut new_config =
                NewSipConfig::new(room_id, modify_sip_config.lobby.unwrap_or_default());

            if let Some(password) = modify_sip_config.password {
                new_config.password = password;
            }

            let config = inventory.create_room_sip_config(new_config).await?;

            let config_resource = SipConfigResource {
                room: room_id,
                sip_id: config.sip_id,
                password: config.password,
                lobby: config.lobby,
            };

            (config_resource, true)
        };

        Ok((sip_config, newly_created))
    }

    pub(crate) async fn delete_sip_config(&self, room_id: RoomId) -> Result<(), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let room = inventory.get_room(room_id).await?;

        require_feature(
            inventory.as_mut(),
            &settings,
            room.created_by,
            &features::CALL_IN_MODULE_FEATURE_ID,
        )
        .await?;

        inventory.delete_room_sip_config(room_id).await?;

        Ok(())
    }
}
