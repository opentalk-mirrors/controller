// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{events::EventId, rooms::RoomId};

/// Specification of a resource provided by the OpenTalk Controller API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    /// An event resource.
    Event(EventId),

    /// An event invite resource.
    EventInvite(EventId),

    /// A room resource.
    Room(RoomId),
}
