// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::schema::event_shared_folders;

#[derive(Insertable)]
#[diesel(table_name = event_shared_folders)]
pub struct NewEventSharedFolder {
    pub event_id: EventId,
    pub path: String,
    pub write_share_id: String,
    pub write_url: String,
    pub write_password: String,
    pub read_share_id: String,
    pub read_url: String,
    pub read_password: String,
}

impl From<inventory::NewEventSharedFolder> for NewEventSharedFolder {
    fn from(
        inventory::NewEventSharedFolder {
            event_id,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }: inventory::NewEventSharedFolder,
    ) -> Self {
        Self {
            event_id,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }
    }
}
