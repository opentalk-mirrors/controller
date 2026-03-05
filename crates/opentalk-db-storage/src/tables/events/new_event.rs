// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::DateTime;
use chrono_tz::Tz;
use diesel::Insertable;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    time::TimeZone,
    users::UserId,
};

use crate::schema::events;

#[derive(Debug, Insertable)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub title: EventTitle,
    pub description: EventDescription,
    pub room: RoomId,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub is_all_day: Option<bool>,
    pub starts_at: Option<DateTime<Tz>>,
    pub starts_at_tz: Option<TimeZone>,
    pub ends_at: Option<DateTime<Tz>>,
    pub ends_at_tz: Option<TimeZone>,
    pub duration_secs: Option<i32>,
    pub recurrence_pattern: Option<String>,
    pub is_adhoc: bool,
    pub tenant_id: TenantId,
    pub show_meeting_details: bool,
}

impl From<inventory::NewEvent> for NewEvent {
    fn from(new_event: inventory::NewEvent) -> Self {
        Self {
            room: new_event.room,
            created_by: new_event.created_by,
            updated_by: new_event.updated_by,
            is_all_day: new_event.is_all_day(),
            starts_at: new_event.starts_at(),
            starts_at_tz: new_event.starts_at_tz(),
            ends_at: new_event.ends_at(),
            ends_at_tz: new_event.ends_at_tz(),
            duration_secs: new_event.duration_secs(),
            recurrence_pattern: new_event.recurrence_pattern().map(ToString::to_string),
            // Note: Title and description are not Copy, value partially moves
            // after here.
            title: new_event.title,
            description: new_event.description,
            is_adhoc: new_event.is_adhoc,
            tenant_id: new_event.tenant_id,
            show_meeting_details: new_event.show_meeting_details,
        }
    }
}
