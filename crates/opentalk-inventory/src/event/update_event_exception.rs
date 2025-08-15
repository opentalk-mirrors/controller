// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    time::TimeZone,
};

use super::EventExceptionKind;

/// Representation of an update to an [`super::EventException`] in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEventException {
    /// The kind of event exception.
    pub kind: Option<EventExceptionKind>,

    /// An optional title changed by the event exception.
    pub title: Option<Option<EventTitle>>,

    /// An optional description changed by the event exception.
    pub description: Option<Option<EventDescription>>,

    /// An optional is_all_day flag changed by the exception.
    pub is_all_day: Option<Option<bool>>,

    /// An optional starts_at timestamp changed by the event exception.
    pub starts_at: Option<Option<DateTime<Tz>>>,

    /// An optional starts_at timezone changed by the event exception.
    pub starts_at_tz: Option<Option<TimeZone>>,

    /// An optional ends_at timestamp changed by the event exception.
    pub ends_at: Option<Option<DateTime<Tz>>>,

    /// An optional ends_at timezone changed by the event exception.
    pub ends_at_tz: Option<Option<TimeZone>>,
}

impl From<opentalk_db_storage::events::UpdateEventException> for UpdateEventException {
    fn from(
        opentalk_db_storage::events::UpdateEventException {
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_db_storage::events::UpdateEventException,
    ) -> Self {
        Self {
            kind: kind.map(Into::into),
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

impl From<UpdateEventException> for opentalk_db_storage::events::UpdateEventException {
    fn from(
        UpdateEventException {
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: UpdateEventException,
    ) -> Self {
        Self {
            kind: kind.map(Into::into),
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
