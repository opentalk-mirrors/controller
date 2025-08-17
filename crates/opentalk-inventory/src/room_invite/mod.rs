// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

#![allow(clippy::module_inception)]

mod new_room_invite;
mod room_invite;
mod room_invite_inventory;
mod room_invite_with_users;
mod update_room_invite;

pub use new_room_invite::NewRoomInvite;
pub use room_invite::RoomInvite;
pub use room_invite_inventory::RoomInviteInventory;
pub use room_invite_with_users::RoomInviteWithUsers;
pub use update_room_invite::UpdateRoomInvite;
