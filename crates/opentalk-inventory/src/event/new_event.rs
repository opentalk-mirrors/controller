// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    events::{EventDescription, EventTitle},
    rooms::RoomId,
    tenants::TenantId,
    users::UserId,
};

use crate::event::new_event_date::NewEventDate;

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

    /// A flag indicating whether this is an ad-hoc event.
    pub is_adhoc: bool,

    /// The id of the tenant to which the event belongs.
    pub tenant_id: TenantId,

    /// A flag indicating whether the details should be shown in the meeting.
    pub show_meeting_details: bool,

    /// Contains all date related information about the event.
    pub date: Option<NewEventDate>,
}
