// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    time::TimeZone,
    users::UserId,
};

use crate::schema::events;

#[derive(Debug, AsChangeset)]
#[diesel(table_name = events)]
pub struct UpdateEvent {
    pub title: Option<EventTitle>,
    pub description: Option<EventDescription>,
    pub updated_by: UserId,
    pub updated_at: DateTime<Utc>,
    pub is_all_day: Option<Option<bool>>,
    pub starts_at: Option<Option<DateTime<Tz>>>,
    pub starts_at_tz: Option<Option<TimeZone>>,
    pub ends_at: Option<Option<DateTime<Tz>>>,
    pub ends_at_tz: Option<Option<TimeZone>>,
    pub duration_secs: Option<Option<i32>>,
    pub recurrence_pattern: Option<Option<String>>,
    pub is_adhoc: Option<bool>,
    pub show_meeting_details: Option<bool>,
}

impl From<inventory::UpdateEvent> for UpdateEvent {
    fn from(update_event: inventory::UpdateEvent) -> Self {
        Self {
            updated_by: update_event.updated_by,
            updated_at: update_event.updated_at.into(),
            is_all_day: Some(update_event.is_all_day()),
            starts_at: Some(update_event.starts_at()),
            starts_at_tz: Some(update_event.starts_at_tz()),
            ends_at: Some(update_event.ends_at()),
            ends_at_tz: Some(update_event.ends_at_tz()),
            duration_secs: Some(update_event.duration_secs()),
            recurrence_pattern: Some(update_event.recurrence_pattern().map(ToString::to_string)),
            is_adhoc: update_event.is_adhoc,
            show_meeting_details: update_event.show_meeting_details,
            title: update_event.title,
            description: update_event.description,
        }
    }
}
