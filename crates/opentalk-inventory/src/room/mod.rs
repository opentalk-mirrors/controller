// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

#![allow(clippy::module_inception)]

mod new_room;
mod room;
mod room_inventory;

pub use new_room::NewRoom;
pub use room::Room;
pub use room_inventory::RoomInventory;
