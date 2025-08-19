// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle},
    time::{TimeZone, Timestamp},
    users::UserId,
};

use super::{EventExceptionId, EventExceptionKind};

/// The representation of an event exception in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventException {
    /// The id of the event exception.
    pub id: EventExceptionId,

    /// The id of the event.
    pub event_id: EventId,

    /// The timestamp for which the exception applies.
    pub exception_date: Timestamp,

    /// The timezone of the exception date.
    pub exception_date_tz: TimeZone,

    /// The id of the use rwho created the event exception.
    pub created_by: UserId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The kind of event exception.
    pub kind: EventExceptionKind,

    /// An optional title changed by the event exception.
    pub title: Option<EventTitle>,

    /// An optional description changed by the event exception.
    pub description: Option<EventDescription>,

    /// An optional is_all_day flag changed by the exception.
    pub is_all_day: Option<bool>,

    /// An optional starts_at timestamp changed by the event exception.
    pub starts_at: Option<Timestamp>,

    /// An optional starts_at timezone changed by the event exception.
    pub starts_at_tz: Option<TimeZone>,

    /// An optional ends_at timestamp changed by the event exception.
    pub ends_at: Option<Timestamp>,

    /// An optional ends_at timezone changed by the event exception.
    pub ends_at_tz: Option<TimeZone>,
}

impl From<opentalk_db_storage::events::EventException> for EventException {
    fn from(
        opentalk_db_storage::events::EventException {
            id,
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            created_at,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_db_storage::events::EventException,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            created_at: created_at.into(),
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at: starts_at.map(Into::into),
            starts_at_tz,
            ends_at: ends_at.map(Into::into),
            ends_at_tz,
        }
    }
}

impl From<EventException> for opentalk_db_storage::events::EventException {
    fn from(
        EventException {
            id,
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            created_at,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: EventException,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            created_at: created_at.into(),
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at: starts_at.map(Into::into),
            starts_at_tz,
            ends_at: ends_at.map(Into::into),
            ends_at_tz,
        }
    }
}
