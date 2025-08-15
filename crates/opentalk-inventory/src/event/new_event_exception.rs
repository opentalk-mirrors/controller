// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle},
    time::{TimeZone, Timestamp},
    users::UserId,
};

use super::EventExceptionKind;

/// The representation of a new event exception that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEventException {
    /// The id of the event.
    pub event_id: EventId,

    /// The timestamp for which the exception applies.
    pub exception_date: Timestamp,

    /// The timezone of the exception date.
    pub exception_date_tz: TimeZone,

    /// The id of the use rwho created the event exception.
    pub created_by: UserId,

    /// The kind of event exception.
    pub kind: EventExceptionKind,

    /// An optional title changed by the event exception.
    pub title: Option<EventTitle>,

    /// An optional description changed by the event exception.
    pub description: Option<EventDescription>,

    /// An optional is_all_day flag changed by the exception.
    pub is_all_day: Option<bool>,

    /// An optional starts_at timestamp changed by the event exception.
    pub starts_at: Option<DateTime<Tz>>,

    /// An optional starts_at timezone changed by the event exception.
    pub starts_at_tz: Option<TimeZone>,

    /// An optional ends_at timestamp changed by the event exception.
    pub ends_at: Option<DateTime<Tz>>,

    /// An optional ends_at timezone changed by the event exception.
    pub ends_at_tz: Option<TimeZone>,
}

impl From<opentalk_db_storage::events::NewEventException> for NewEventException {
    fn from(
        opentalk_db_storage::events::NewEventException {
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_db_storage::events::NewEventException,
    ) -> Self {
        Self {
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }
    }
}

impl From<NewEventException> for opentalk_db_storage::events::NewEventException {
    fn from(
        NewEventException {
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: NewEventException,
    ) -> Self {
        Self {
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }
    }
}
