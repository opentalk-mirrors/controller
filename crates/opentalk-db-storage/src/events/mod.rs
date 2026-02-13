// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

pub use crate::tables::{
    event_email_invites as email_invites,
    event_exceptions::{EventException, NewEventException, UpdateEventException},
    event_favorites::{EventFavorite, NewEventFavorite},
    event_invites::{EventInvite, NewEventInvite, UpdateEventInvite},
    event_shared_folders as shared_folders,
    event_training_participation_report_parameter_sets::{
        EventTrainingParticipationReportParameterSet,
        UpdateEventTrainingParticipationReportParameterSet,
    },
    events::{Event, NewEvent, UpdateEvent},
}; // TODO: rm -f

pub struct GetEventsCursor {
    pub from_id: EventId,
    pub from_created_at: DateTime<Utc>,
    pub from_starts_at: Option<DateTime<Utc>>,
}

impl From<GetEventsCursor> for inventory::GetEventsCursor {
    fn from(
        GetEventsCursor {
            from_id,
            from_created_at,
            from_starts_at,
        }: GetEventsCursor,
    ) -> Self {
        Self::new(
            from_id,
            from_created_at.into(),
            from_starts_at.map(Into::into),
        )
    }
}

impl From<inventory::GetEventsCursor> for GetEventsCursor {
    fn from(value: inventory::GetEventsCursor) -> Self {
        let (from_id, from_created_at, from_starts_at) = value.into();
        Self {
            from_id,
            from_created_at: from_created_at.into(),
            from_starts_at: from_starts_at.map(Into::into),
        }
    }
}

impl GetEventsCursor {
    pub fn from_last_event_in_query(event: &Event) -> Self {
        Self {
            from_id: event.id,
            from_created_at: event.created_at,
            from_starts_at: event.starts_at,
        }
    }
}

pub struct GetEventExceptionsCursor {
    pub from_id: EventId,
    pub from_created_at: DateTime<Utc>,
    pub from_starts_at: Option<DateTime<Utc>>,
    pub from_exception_date: DateTime<Utc>,
}

impl From<GetEventExceptionsCursor> for inventory::GetEventExceptionsCursor {
    fn from(
        GetEventExceptionsCursor {
            from_id,
            from_created_at,
            from_starts_at,
            from_exception_date,
        }: GetEventExceptionsCursor,
    ) -> Self {
        Self::new(
            from_id,
            from_created_at.into(),
            from_starts_at.map(Into::into),
            from_exception_date.into(),
        )
    }
}

impl From<inventory::GetEventExceptionsCursor> for GetEventExceptionsCursor {
    fn from(value: inventory::GetEventExceptionsCursor) -> Self {
        let (from_id, from_created_at, from_starts_at, from_exception_date) = value.into();
        Self {
            from_id,
            from_created_at: from_created_at.into(),
            from_starts_at: from_starts_at.map(Into::into),
            from_exception_date: from_exception_date.into(),
        }
    }
}

impl GetEventExceptionsCursor {
    pub fn from_last_event_in_query(exception: &EventException) -> Self {
        Self {
            from_id: exception.event_id,
            from_created_at: exception.created_at,
            from_starts_at: exception.starts_at,
            from_exception_date: exception.exception_date,
        }
    }
}
