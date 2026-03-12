// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::str::FromStr;

use opentalk_controller_utils::CaptureApiError;
use opentalk_roomserver_types::{
    breakout::breakout_id::BreakoutId,
    client_parameters::{ClientKind, ClientParameters, Role},
    room_kind::RoomKind,
};
use opentalk_signaling_core::{Participant, assets::verify_storage_usage};
use opentalk_types_api_internal::recording::RecordingTarget;
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::{RoomResource, by_room_id::RoomserverStartResponseBody},
    services::{PostServiceStartResponseBody, recording::PostRecordingStartRequestBody},
};
use opentalk_types_common::roomserver::{DEVICE_SECRET_MIN_LENGTH, DeviceSecret};
use rand::RngExt;

use crate::{
    ControllerBackend, ToUserProfile, signaling::ticket::start_or_continue_signaling_session,
};

impl ControllerBackend {
    pub(crate) async fn start_recording(
        &self,
        body: PostRecordingStartRequestBody,
    ) -> Result<PostServiceStartResponseBody, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;
        let mut volatile = self.volatile.clone();

        if settings
            .rabbit_mq
            .as_ref()
            .and_then(|c| c.recording_task_queue.as_ref())
            .is_none()
        {
            return Err(ApiError::not_found().into());
        }

        let room = inventory.get_room(body.room_id).await?;

        let _ = verify_storage_usage(inventory.as_mut(), room.created_by).await?;

        let (ticket, resumption) = start_or_continue_signaling_session(
            &mut volatile,
            Participant::Recorder,
            room.id,
            body.breakout_room,
            None,
        )
        .await?;

        Ok(PostServiceStartResponseBody { ticket, resumption })
    }

    pub(crate) async fn start_recording_roomserver(
        &self,
        body: RecordingTarget,
    ) -> Result<RoomserverStartResponseBody, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (room, creator) = inventory.get_room_with_creator(body.room_id).await?;

        let _ = verify_storage_usage(inventory.as_mut(), room.created_by).await?;

        let room_resource = RoomResource {
            id: room.id,
            created_by: creator.to_public_user_profile(&settings),
            created_at: room.created_at,
            password: room.password,
            waiting_room: room.waiting_room,
        };

        let device_secret = rand::rng()
            .sample_iter(rand::distr::Alphanumeric)
            .take(DEVICE_SECRET_MIN_LENGTH)
            .map(|c| c as char)
            .collect::<String>();
        let device_secret = DeviceSecret::from_str(&device_secret)
            .expect("String is at least DEVICE_SECRET_MIN_LENGTH long");

        let room = if let Some(breakout_id) = body.breakout_room {
            RoomKind::Breakout(BreakoutId::from(breakout_id))
        } else {
            RoomKind::Main
        };

        let client_parameters = ClientParameters {
            device_secret,
            kind: ClientKind::Recorder { room },
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
}
