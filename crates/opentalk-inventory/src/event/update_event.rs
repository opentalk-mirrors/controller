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

use crate::{UpdateEventRecurrence, event::UpdateEventDate};

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

    /// A flag indicating whether this is an ad-hoc event.
    pub is_adhoc: Option<bool>,

    /// A flag indicating whether the details should be shown in the meeting.
    pub show_meeting_details: Option<bool>,

    /// Contains all date related information about the event.
    pub date: Option<UpdateEventDate>,
}

impl UpdateEvent {
    /// Returns `true` if this [`UpdateEvent`] has no date field associated.
    pub fn is_time_independent(&self) -> bool {
        self.date.is_none()
    }

    /// Returns the `date` of this [`UpdateEvent`].
    pub fn date(&self) -> Option<&UpdateEventDate> {
        self.date.as_ref()
    }

    /// Returns the `is_all_day` of this [`UpdateEvent`].
    pub fn is_all_day(&self) -> Option<bool> {
        self.date().and_then(|date| date.is_all_day)
    }

    /// Returns the `starts_at` of this [`UpdateEvent`].
    pub fn starts_at(&self) -> Option<DateTime<Tz>> {
        self.date().and_then(|date| date.starts_at)
    }

    /// Returns the `ends_at` of this [`UpdateEvent`].
    pub fn ends_at(&self) -> Option<DateTime<Tz>> {
        self.date().and_then(|date| date.ends_at)
    }

    /// Returns the `starts_at_tz` of this [`UpdateEvent`].
    pub fn starts_at_tz(&self) -> Option<TimeZone> {
        self.date().and_then(|date| date.starts_at_tz)
    }

    /// Returns the `ends_at_tz` of this [`UpdateEvent`].
    pub fn ends_at_tz(&self) -> Option<TimeZone> {
        self.date().and_then(|date| date.ends_at_tz)
    }

    /// Returns the `recurrence` of this [`UpdateEvent`].
    pub fn recurrence(&self) -> Option<&UpdateEventRecurrence> {
        self.date().and_then(|date| date.recurrence.as_ref())
    }

    /// Returns the `duration_secs` of this [`UpdateEvent`].
    pub fn duration_secs(&self) -> Option<i32> {
        self.recurrence()
            .and_then(|recurrence| recurrence.duration_secs)
    }

    /// Returns the `recurrence_pattern` of this [`UpdateEvent`].
    pub fn recurrence_pattern(&self) -> Option<&str> {
        self.recurrence()
            .and_then(|recurrence| recurrence.recurrence_pattern.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;
    use opentalk_types_common::utils::ExampleData;

    use super::*;

    #[test]
    fn is_time_independent() {
        let time_independent_event = UpdateEvent {
            title: Some(EventTitle::example_data()),
            description: Some(EventDescription::example_data()),
            updated_by: UserId::example_data(),
            updated_at: Timestamp::example_data(),
            is_adhoc: Some(false),
            show_meeting_details: Some(false),
            date: None,
        };

        assert!(time_independent_event.is_time_independent());

        let time_dependent_event = UpdateEvent {
            title: Some(EventTitle::example_data()),
            description: Some(EventDescription::example_data()),
            updated_by: UserId::example_data(),
            updated_at: Timestamp::example_data(),
            is_adhoc: Some(false),
            show_meeting_details: Some(false),
            date: Some(UpdateEventDate {
                is_all_day: Some(false),
                starts_at: Some(
                    Tz::default()
                        .with_ymd_and_hms(2024, 7, 20, 14, 16, 19)
                        .unwrap(),
                ),
                starts_at_tz: Some(TimeZone::default()),
                ends_at: Some(
                    Tz::default()
                        .with_ymd_and_hms(2024, 7, 20, 14, 16, 19)
                        .unwrap(),
                ),
                ends_at_tz: Some(TimeZone::default()),
                recurrence: None,
            }),
        };

        assert!(!time_dependent_event.is_time_independent());
    }
}
