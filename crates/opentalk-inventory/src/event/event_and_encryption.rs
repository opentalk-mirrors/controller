// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::events::EventInfo;

use super::Event;

pub(crate) struct EventAndEncryption<'a>(pub &'a Event, pub bool);

impl<'a> From<EventAndEncryption<'a>> for EventInfo {
    fn from(value: EventAndEncryption<'a>) -> Self {
        let EventAndEncryption(event, e2e_encryption) = value;
        EventInfo {
            id: event.id,
            room_id: event.room,
            title: event.title.clone(),
            is_adhoc: event.is_adhoc,
            meeting_details: None,
            e2e_encryption,
        }
    }
}
