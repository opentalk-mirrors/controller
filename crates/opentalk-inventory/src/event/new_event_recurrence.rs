// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// Contains information about the recurrence of a new event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEventRecurrence {
    /// Only for recurring events, since ends_at contains the information
    /// about the last occurrence of the recurring series this duration
    /// value.
    ///
    /// MUST be used to calculate the event instances length.
    pub duration_secs: i32,
    /// The recurrence pattern for recurring events.
    pub recurrence_pattern: String,
}
