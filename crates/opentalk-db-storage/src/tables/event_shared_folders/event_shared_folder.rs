// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::{schema::event_shared_folders, tables::events::Event};

#[derive(Clone, Debug, PartialEq, Eq, Associations, Identifiable, Queryable)]
#[diesel(table_name = event_shared_folders)]
#[diesel(primary_key(event_id))]
#[diesel(belongs_to(Event))]
pub struct EventSharedFolder {
    pub event_id: EventId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub path: String,
    pub write_share_id: String,
    pub write_url: String,
    pub write_password: String,
    pub read_share_id: String,
    pub read_url: String,
    pub read_password: String,
}

impl From<EventSharedFolder> for inventory::EventSharedFolder {
    fn from(
        EventSharedFolder {
            event_id,
            created_at,
            updated_at,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }: EventSharedFolder,
    ) -> Self {
        Self {
            event_id,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
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

impl From<inventory::EventSharedFolder> for EventSharedFolder {
    fn from(
        inventory::EventSharedFolder {
            event_id,
            created_at,
            updated_at,
            path,
            write_share_id,
            write_url,
            write_password,
            read_share_id,
            read_url,
            read_password,
        }: inventory::EventSharedFolder,
    ) -> Self {
        Self {
            event_id,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
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
