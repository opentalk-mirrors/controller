// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    time::{TimeZone, Timestamp},
    users::UserId,
};

/// The representation of an event in the inventory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// The id of the event.
    pub id: EventId,

    /// The serial id of the event.
    pub id_serial: i64,

    /// The title of the event.
    pub title: EventTitle,

    /// The description of the event.
    pub description: EventDescription,

    /// The id of the room associated with the event.
    pub room: RoomId,

    /// The id of the user who created the event.
    pub created_by: UserId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The id of the user last updated the event.
    pub updated_by: UserId,

    /// The updated timestamp.
    pub updated_at: Timestamp,

    /// A flag indicating whether the event is time-independent.
    pub is_time_independent: bool,

    /// A flag indicating whether this is an all-day event.
    pub is_all_day: Option<bool>,

    /// start datetime of the event
    pub starts_at: Option<Timestamp>,

    /// timezone of the start-datetime of the event
    pub starts_at_tz: Option<TimeZone>,

    /// end datetime of the event
    ///
    /// For recurring events contains the timestamp of the last occurrence
    pub ends_at: Option<Timestamp>,

    /// timezone of the ends_at datetime
    pub ends_at_tz: Option<TimeZone>,

    /// Only for recurring events, since ends_at contains the information
    /// about the last occurrence of the recurring series this duration value
    /// MUST be used to calculate the event instances length
    pub duration_secs: Option<i32>,

    /// A flag indicating whether this is a recurring event.
    pub is_recurring: Option<bool>,

    /// The recurrence pattern for recurring events.
    pub recurrence_pattern: Option<String>,

    /// A flag indicating whether this is an ad-hoc event.
    pub is_adhoc: bool,

    /// The id of the tenant to which the event belongs.
    pub tenant_id: TenantId,

    /// The revision number of the event, incremented with each change.
    pub revision: i32,

    /// A flag indicating whether the details should be shown in the meeting.
    pub show_meeting_details: bool,
}

impl Event {
    /// Returns the ends_at value of the first occurrence of the event
    pub fn ends_at_of_first_occurrence(&self) -> Option<(Timestamp, TimeZone)> {
        if self.is_recurring.unwrap_or_default() {
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
