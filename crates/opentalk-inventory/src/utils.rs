// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Some helper utilities for interacting with the data storage.

use opentalk_types_common::{
    call_in::CallInInfo,
    events::{EventInfo, MeetingDetails},
    features::{CALL_IN_FEATURE_ID, GUESTS_ALLOWED_FEATURE_ID},
    modules::CORE_MODULE_ID,
    rooms::RoomId,
    streaming::get_public_urls_from_room_streaming_targets,
    tariffs::TariffResource,
};

use crate::{Event, Inventory, Result, event::EventAndEncryption};

/// Build the user-facing event info for a room.
pub async fn build_event_info(
    inventory: &mut dyn Inventory,
    call_in_tel: Option<String>,
    room_id: RoomId,
    e2e_encryption: bool,
    event: &Event,
    tariff: &TariffResource,
) -> Result<EventInfo> {
    let event_info = if event.show_meeting_details {
        let invite = if tariff.has_feature_enabled(&CORE_MODULE_ID, &GUESTS_ALLOWED_FEATURE_ID) {
            inventory.get_valid_invite_for_room(room_id).await?
        } else {
            None
        };

        let call_in = if let Some(call_in_tel) = call_in_tel {
            if e2e_encryption || !tariff.has_feature_enabled(&CORE_MODULE_ID, &CALL_IN_FEATURE_ID) {
                None
            } else {
                inventory
                    .get_room_sip_config(room_id)
                    .await?
                    .map(|sip_config| CallInInfo {
                        tel: call_in_tel,
                        id: sip_config.sip_id,
                        password: sip_config.password,
                    })
            }
        } else {
            None
        };

        let streaming_links = if !e2e_encryption {
            let streaming_targets = inventory.get_room_streaming_targets(room_id).await?;
            get_public_urls_from_room_streaming_targets(streaming_targets).await
        } else {
            vec![]
        };

        EventInfo::from(EventAndEncryption(event, e2e_encryption)).with_meeting_details(
            MeetingDetails {
                invite_code_id: invite.map(|invite| invite.invite_code),
                call_in,
                streaming_links,
            },
        )
    } else {
        EventInfo::from(EventAndEncryption(event, e2e_encryption))
    };

    Ok(event_info)
}
