// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    time::{TimeZone, Timestamp},
    users::UserId,
};

/// Representation of an update to an [`super::Event`] in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEvent {
    /// The title of the event.
    pub title: Option<EventTitle>,

    /// The description of the event.
    pub description: Option<EventDescription>,

    /// The id of the user last updated the event.
    pub updated_by: UserId,

    /// The updated timestamp.
    pub updated_at: Timestamp,

    /// A flag indicating whether the event is time-independent.
    pub is_time_independent: Option<bool>,

    /// A flag indicating whether this is an all-day event.
    pub is_all_day: Option<Option<bool>>,

    /// start datetime of the event
    pub starts_at: Option<Option<DateTime<Tz>>>,

    /// timezone of the start-datetime of the event
    pub starts_at_tz: Option<Option<TimeZone>>,

    /// end datetime of the event
    ///
    /// For recurring events contains the timestamp of the last occurrence
    pub ends_at: Option<Option<DateTime<Tz>>>,

    /// timezone of the ends_at datetime
    pub ends_at_tz: Option<Option<TimeZone>>,

    /// Only for recurring events, since ends_at contains the information
    /// about the last occurrence of the recurring series this duration value
    /// MUST be used to calculate the event instances length
    pub duration_secs: Option<Option<i32>>,

    /// The recurrence pattern for recurring events.
    pub recurrence_pattern: Option<Option<String>>,

    /// A flag indicating whether this is an ad-hoc event.
    pub is_adhoc: Option<bool>,

    /// A flag indicating whether the details should be shown in the meeting.
    pub show_meeting_details: Option<bool>,
}
