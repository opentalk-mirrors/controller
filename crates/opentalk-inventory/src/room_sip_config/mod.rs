// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

#![allow(clippy::module_inception)]

mod new_room_sip_config;
mod room_sip_config;
mod room_sip_config_inventory;

pub use new_room_sip_config::NewRoomSipConfig;
pub use room_sip_config::RoomSipConfig;
pub use room_sip_config_inventory::RoomSipConfigInventory;
