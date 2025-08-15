// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The id of an event invite.
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
pub struct EventInviteId(uuid::Uuid);

impl From<EventInviteId> for opentalk_db_storage::events::EventInviteId {
    fn from(EventInviteId(id): EventInviteId) -> Self {
        Self::from(id)
    }
}

impl From<opentalk_db_storage::events::EventInviteId> for EventInviteId {
    fn from(value: opentalk_db_storage::events::EventInviteId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}
