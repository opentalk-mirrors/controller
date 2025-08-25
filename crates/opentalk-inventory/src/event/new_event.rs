// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    time::TimeZone,
    users::UserId,
};

/// The representation of a new event that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    /// The title of the event.
    pub title: EventTitle,

    /// The description of the event.
    pub description: EventDescription,

    /// The id of the room associated with the event.
    pub room: RoomId,

    /// The id of the user who created the event.
    pub created_by: UserId,

    /// The id of the user last updated the event.
    pub updated_by: UserId,

    /// A flag indicating whether the event is time-independent.
    pub is_time_independent: bool,

    /// A flag indicating whether this is an all-day event.
    pub is_all_day: Option<bool>,

    /// start datetime of the event
    pub starts_at: Option<DateTime<Tz>>,

    /// timezone of the start-datetime of the event
    pub starts_at_tz: Option<TimeZone>,

    /// end datetime of the event
    ///
    /// For recurring events contains the timestamp of the last occurrence
    pub ends_at: Option<DateTime<Tz>>,

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

    /// A flag indicating whether the details should be shown in the meeting.
    pub show_meeting_details: bool,
}
