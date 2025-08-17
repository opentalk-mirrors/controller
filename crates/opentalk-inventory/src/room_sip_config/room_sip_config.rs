// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    call_in::{CallInId, CallInPassword},
    rooms::RoomId,
};

/// The representation of a room SIP config in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSipConfig {
    /// The id of the config.
    pub id: i64,

    /// The id of the room to which the SIP config belongs.
    pub room: RoomId,

    /// The call-in id for the room.
    pub sip_id: CallInId,

    /// The call-in password for the room.
    pub password: CallInPassword,

    /// A flag indicating whether SIP participants should be sent to the lobby first.
    pub lobby: bool,
}

impl From<opentalk_db_storage::sip_configs::SipConfig> for RoomSipConfig {
    fn from(
        opentalk_db_storage::sip_configs::SipConfig {
            id,
            room,
            sip_id,
            password,
            lobby,
        }: opentalk_db_storage::sip_configs::SipConfig,
    ) -> Self {
        Self {
            id,
            room,
            sip_id,
            password,
            lobby,
        }
    }
}

impl From<RoomSipConfig> for opentalk_db_storage::sip_configs::SipConfig {
    fn from(
        RoomSipConfig {
            id,
            room,
            sip_id,
            password,
            lobby,
        }: RoomSipConfig,
    ) -> Self {
        Self {
            id,
            room,
            sip_id,
            password,
            lobby,
        }
    }
}
