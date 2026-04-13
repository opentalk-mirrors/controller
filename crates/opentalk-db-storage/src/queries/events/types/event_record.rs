// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::tables::{event_dates::EventDate, events::Event};

/// Composite type that combines both [`Event`] and an optional [`EventDate`]. This is used for the
/// translation into [`opentalk_inventory::Event`].
#[derive(Debug, Clone, PartialEq, Queryable)]
pub struct EventRecord {
    /// Core event related fields.
    event: Event,
    /// Event date related fields.
    date: Option<EventDate>,
}

impl EventRecord {
    /// Creates a new [`EventRecord`].
    pub fn new(event: Event, date: Option<EventDate>) -> Self {
        Self { event, date }
    }

    /// Returns a reference to the event of this [`EventRecord`].
    pub fn event(&self) -> &Event {
        &self.event
    }

    /// Returns the id of this [`EventRecord`].
    pub fn id(&self) -> EventId {
        self.event().id
    }

    /// Returns the date of this [`EventRecord`].
    pub fn date(&self) -> Option<&EventDate> {
        self.date.as_ref()
    }
}

impl From<inventory::Event> for EventRecord {
    fn from(
        inventory::Event {
            id,
            id_serial,
            title,
            description,
            room,
            created_by,
            created_at,
            updated_by,
            updated_at,
            is_adhoc,
            tenant_id,
            revision,
            show_meeting_details,
            date,
        }: inventory::Event,
    ) -> Self {
        let date = date.map(
            |inventory::EventDate {
                 is_all_day,
                 starts_at,
                 starts_at_tz,
                 ends_at,
                 ends_at_tz,
                 recurrence,
             }| EventDate {
                event_id: id,
                starts_at: starts_at.into(),
                starts_at_tz,
                ends_at: ends_at.into(),
                ends_at_tz,
                is_all_day,
                duration_secs: recurrence.as_ref().map(|r| r.duration_secs),
                recurrence_pattern: recurrence.map(|r| r.recurrence_pattern),
            },
        );

        Self {
            event: Event {
                id,
                id_serial: id_serial.into(),
                title,
                description,
                room,
                created_by,
                created_at: created_at.into(),
                updated_by,
                updated_at: updated_at.into(),
                is_adhoc,
                tenant_id,
                revision,
                show_meeting_details,
            },
            date,
        }
    }
}

impl From<EventRecord> for inventory::Event {
    fn from(
        EventRecord {
            event:
                Event {
                    id,
                    id_serial,
                    title,
                    description,
                    room,
                    created_by,
                    created_at,
                    updated_by,
                    updated_at,
                    is_adhoc,
                    tenant_id,
                    revision,
                    show_meeting_details,
                },
            date,
        }: EventRecord,
    ) -> inventory::Event {
        Self {
            id,
            id_serial: id_serial.into(),
            title,
            description,
            room,
            created_by,
            created_at: created_at.into(),
            updated_by,
            updated_at: updated_at.into(),
            is_adhoc,
            tenant_id,
            revision,
            show_meeting_details,
            date: date.map(Into::into),
        }
    }
}
