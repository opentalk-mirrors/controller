// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use diesel::prelude::*;
use opentalk_inventory as inventory;
use opentalk_types_common::time::TimeZone;

use crate::schema::event_dates;

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = event_dates)]
pub struct UpdateEventDate {
    /// Start datetime of the event.
    starts_at: Option<DateTime<Tz>>,
    /// Timezone of the `starts_at` datetime.
    starts_at_tz: Option<TimeZone>,
    /// End datetime of the event.
    ///
    /// For recurring events contains the timestamp of the last occurrence.
    ends_at: Option<DateTime<Tz>>,
    /// Timezone of the `ends_at` datetime.
    ends_at_tz: Option<TimeZone>,
    /// Denotes whether an event is all day, meaning it starts at 00:00 and ends at 00:00 the
    /// following day.
    is_all_day: Option<bool>,
    /// Only for recurring events, since ends_at contains the information about the last occurrence
    /// of the recurring series this duration value.
    ///
    /// MUST be used to calculate the event instances length.
    duration_secs: Option<Option<i32>>,
    /// Recurrence pattern of the event.
    recurrence_pattern: Option<Option<String>>,
}

impl From<UpdateEventDate> for inventory::UpdateEventDate {
    fn from(
        UpdateEventDate {
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            is_all_day,
            duration_secs,
            recurrence_pattern,
        }: UpdateEventDate,
    ) -> Self {
        Self {
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            recurrence: duration_secs.zip(recurrence_pattern).map(
                |(duration_secs, recurrence_pattern)| inventory::UpdateEventRecurrence {
                    duration_secs,
                    recurrence_pattern,
                },
            ),
        }
    }
}

impl From<inventory::UpdateEventDate> for UpdateEventDate {
    fn from(
        inventory::UpdateEventDate {
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            is_all_day,
            recurrence,
        }: inventory::UpdateEventDate,
    ) -> Self {
        Self {
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            is_all_day,
            duration_secs: recurrence.as_ref().map(|r| r.duration_secs),
            recurrence_pattern: recurrence.map(|r| r.recurrence_pattern),
        }
    }
}
