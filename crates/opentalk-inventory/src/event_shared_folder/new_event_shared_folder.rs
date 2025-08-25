// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::events::EventId;

/// The representation of a new event shared folder that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEventSharedFolder {
    /// The title of the event.
    pub event_id: EventId,

    /// The path of the shared folder.
    pub path: String,

    /// The share id for the write share.
    pub write_share_id: String,

    /// The url for the write share.
    pub write_url: String,

    /// The password for the write share.
    pub write_password: String,

    /// The share id for the read share.
    pub read_share_id: String,

    /// The url for the read share.
    pub read_url: String,

    /// The password for the read share.
    pub read_password: String,
}
