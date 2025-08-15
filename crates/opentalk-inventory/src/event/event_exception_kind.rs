// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The kinds of event exceptions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventExceptionKind {
    /// The event instance is modified.
    Modified,

    /// The event instance is cancelled.
    Cancelled,
}

impl From<opentalk_db_storage::events::EventExceptionKind> for EventExceptionKind {
    fn from(value: opentalk_db_storage::events::EventExceptionKind) -> Self {
        use opentalk_db_storage::events::EventExceptionKind as DbEventExceptionKind;
        match value {
            DbEventExceptionKind::Modified => Self::Modified,
            DbEventExceptionKind::Cancelled => Self::Cancelled,
        }
    }
}

impl From<EventExceptionKind> for opentalk_db_storage::events::EventExceptionKind {
    fn from(value: EventExceptionKind) -> Self {
        match value {
            EventExceptionKind::Modified => Self::Modified,
            EventExceptionKind::Cancelled => Self::Cancelled,
        }
    }
}
