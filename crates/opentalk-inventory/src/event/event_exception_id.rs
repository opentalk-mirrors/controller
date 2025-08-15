// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The id of an event exception.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct EventExceptionId(uuid::Uuid);

impl From<EventExceptionId> for opentalk_db_storage::events::EventExceptionId {
    fn from(EventExceptionId(id): EventExceptionId) -> Self {
        Self::from(id)
    }
}

impl From<opentalk_db_storage::events::EventExceptionId> for EventExceptionId {
    fn from(value: opentalk_db_storage::events::EventExceptionId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}
