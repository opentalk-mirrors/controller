// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_inventory as inventory;

use crate::tables::{event_dates::UpdateEventDate, events::UpdateEvent};

/// Composite type that combines both [`UpdateEvent`] and an optional [`UpdateEventDate`]. This is used for the
/// translation into [`inventory::Event`].
#[derive(Debug, Clone)]
pub struct UpdateEventRecord {
    /// Core event related fields.
    event: UpdateEvent,
    /// Event date related fields.
    date: Option<UpdateEventDate>,
}

impl UpdateEventRecord {
    /// Returns the date of this [`UpdateEventRecord`].
    pub fn date(&self) -> Option<&UpdateEventDate> {
        self.date.as_ref()
    }

    /// Returns a reference to the event of this [`UpdateEventRecord`].
    pub fn event(&self) -> &UpdateEvent {
        &self.event
    }
}

impl From<inventory::UpdateEvent> for UpdateEventRecord {
    fn from(
        inventory::UpdateEvent {
            title,
            description,
            updated_by,
            updated_at,
            is_adhoc,
            show_meeting_details,
            date,
        }: inventory::UpdateEvent,
    ) -> UpdateEventRecord {
        Self {
            event: UpdateEvent {
                title,
                description,
                updated_by,
                updated_at: updated_at.into(),
                is_adhoc,
                show_meeting_details,
            },
            date: date.map(Into::into),
        }
    }
}
