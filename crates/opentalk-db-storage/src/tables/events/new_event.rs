// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::Insertable;
use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    users::UserId,
};

use crate::schema::events;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub title: EventTitle,
    pub description: EventDescription,
    pub room: RoomId,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub is_adhoc: bool,
    pub tenant_id: TenantId,
    pub show_meeting_details: bool,
}

impl From<opentalk_inventory::NewEvent> for NewEvent {
    fn from(new_event: opentalk_inventory::NewEvent) -> Self {
        Self {
            title: new_event.title,
            description: new_event.description,
            room: new_event.room,
            created_by: new_event.created_by,
            updated_by: new_event.updated_by,
            is_adhoc: new_event.is_adhoc,
            tenant_id: new_event.tenant_id,
            show_meeting_details: new_event.show_meeting_details,
        }
    }
}
