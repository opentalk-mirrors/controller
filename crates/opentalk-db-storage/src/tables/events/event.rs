// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    time::TimeZone,
    users::UserId,
};
use redis_args::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::{
    schema::events,
    tables::{events::EventSerialId, rooms::Room},
    users::User,
};

#[derive(
    Associations,
    Clone,
    Debug,
    Deserialize,
    Eq,
    FromRedisValue,
    Identifiable,
    PartialEq,
    Queryable,
    Serialize,
    ToRedisArgs,
)]
#[diesel(table_name = events)]
#[diesel(belongs_to(User, foreign_key = created_by))]
#[diesel(belongs_to(Room, foreign_key = room))]
#[to_redis_args(serde)]
#[from_redis_value(serde)]
pub struct Event {
    pub id: EventId,
    pub id_serial: EventSerialId,
    pub title: EventTitle,
    pub description: EventDescription,
    pub room: RoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_by: UserId,
    pub updated_at: DateTime<Utc>,
    pub is_all_day: Option<bool>,
    /// start datetime of the event
    pub starts_at: Option<DateTime<Utc>>,
    /// timezone of the start-datetime of the event
    pub starts_at_tz: Option<TimeZone>,
    /// end datetime of the event
    ///
    /// For recurring events contains the timestamp of the last occurrence
    pub ends_at: Option<DateTime<Utc>>,
    /// timezone of the ends_at datetime
    pub ends_at_tz: Option<TimeZone>,
    /// Only for recurring events, since ends_at contains the information
    /// about the last occurrence of the recurring series this duration value.
    ///
    /// MUST be used to calculate the event instances length.
    pub duration_secs: Option<i32>,
    pub recurrence_pattern: Option<String>,
    pub is_adhoc: bool,
    pub tenant_id: TenantId,
    pub revision: i32,
    pub show_meeting_details: bool,
}

impl From<Event> for inventory::Event {
    fn from(
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
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            duration_secs,
            recurrence_pattern,
            is_adhoc,
            tenant_id,
            revision,
            show_meeting_details,
        }: Event,
    ) -> Self {
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
            date: (|| {
                Some(inventory::EventDate {
                    is_all_day: is_all_day?,
                    starts_at: starts_at?.into(),
                    starts_at_tz: starts_at_tz?,
                    ends_at: ends_at?.into(),
                    ends_at_tz: ends_at_tz?,
                    recurrence: recurrence_pattern.zip(duration_secs).map(
                        |(recurrence_pattern, duration_secs)| inventory::EventRecurrence {
                            recurrence_pattern,
                            duration_secs,
                        },
                    ),
                })
            })(),
        }
    }
}

impl From<&Event> for inventory::Event {
    fn from(value: &Event) -> Self {
        Self::from(value.clone())
    }
}

impl From<inventory::Event> for Event {
    fn from(event: inventory::Event) -> Self {
        Self {
            id: event.id,
            id_serial: event.id_serial.into(),
            room: event.room,
            created_by: event.created_by,
            created_at: event.created_at.into(),
            updated_by: event.updated_by,
            updated_at: event.updated_at.into(),
            is_all_day: event.is_all_day(),
            starts_at: event.starts_at().map(Into::into),
            starts_at_tz: event.starts_at_tz(),
            ends_at: event.ends_at().map(Into::into),
            ends_at_tz: event.ends_at_tz(),
            duration_secs: event.duration_secs(),
            recurrence_pattern: event.recurrence_pattern().map(ToString::to_string),
            is_adhoc: event.is_adhoc,
            tenant_id: event.tenant_id,
            revision: event.revision,
            show_meeting_details: event.show_meeting_details,
            // Note: Title and descrioption are note Copy, event partially moves
            // after here.
            title: event.title,
            description: event.description,
        }
    }
}

impl From<&inventory::Event> for Event {
    fn from(value: &inventory::Event) -> Self {
        Self::from(value.clone())
    }
}

impl Event {
    /// Returns the ends_at value of the first occurrence of the event
    pub fn ends_at_of_first_occurrence(&self) -> Option<(DateTime<Utc>, TimeZone)> {
        if self.recurrence_pattern.is_some() {
            // Recurring events have the last occurrence of the recurrence saved in the ends_at fields
            // So we get the starts_at_dt and add the duration_secs field to it
            if let (Some(starts_at_dt), Some(dur), Some(tz)) =
                (self.starts_at, self.duration_secs, self.ends_at_tz)
            {
                Some((starts_at_dt + chrono::Duration::seconds(i64::from(dur)), tz))
            } else {
                None
            }
        } else if let (Some(dt), Some(tz)) = (self.ends_at, self.ends_at_tz) {
            // Non recurring events just directly use the ends_at field from the db
            Some((dt, tz))
        } else {
            None
        }
    }
}
