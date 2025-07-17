// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_utils::CaptureApiError;
use opentalk_signaling_core::{Participant, assets::verify_storage_usage};
use opentalk_types_api_v1::{
    error::ApiError,
    services::{PostServiceStartResponseBody, recording::PostRecordingStartRequestBody},
};

use crate::{ControllerBackend, signaling::ticket::start_or_continue_signaling_session};

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

        verify_storage_usage(inventory.as_mut(), room.created_by).await?;

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
}
