// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;

use crate::schema::event_recurrences;

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = event_recurrences)]
pub struct UpdateEventRecurrence {
    /// Only for recurring events, since ends_at contains the information about the last occurrence
    /// of the recurring series this duration value.
    ///
    /// MUST be used to calculate the event instances length.
    pub duration_secs: Option<i32>,
    /// Recurrence pattern of the event.
    pub recurrence_pattern: Option<String>,
}
