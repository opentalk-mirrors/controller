// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::tables::{event_dates::NewEventDate, events::NewEvent};

#[derive(Debug, Clone)]
pub struct NewEventRecord {
    /// New event to be inserted into the db.
    event: NewEvent,
    /// Raw event date without event id.
    raw_date: Option<inventory::NewEventDate>,
}

impl NewEventRecord {
    /// Construct the [NewEventDate] with [EventId]; returns [None] if `raw_date` is
    /// [None].
    pub fn build_date(self, event_id: EventId) -> Option<NewEventDate> {
        self.raw_date
            .map(|raw_date| NewEventDate::new(raw_date, event_id))
    }

    /// Returns a reference to the event of this [`NewEventRecord`].
    pub fn event(&self) -> &NewEvent {
        &self.event
    }
}

impl From<inventory::NewEvent> for NewEventRecord {
    fn from(
        inventory::NewEvent {
            title,
            description,
            room,
            created_by,
            updated_by,
            is_adhoc,
            tenant_id,
            show_meeting_details,
            date,
        }: inventory::NewEvent,
    ) -> Self {
        Self {
            event: NewEvent {
                title,
                description,
                room,
                created_by,
                updated_by,
                is_adhoc,
                tenant_id,
                show_meeting_details,
            },
            raw_date: date,
        }
    }
}
