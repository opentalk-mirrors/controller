// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains streaming targets table structs

mod room_streaming_target_new;
mod room_streaming_target_record;
mod update_room_streaming_target;

pub use room_streaming_target_new::RoomStreamingTargetNew;
pub use room_streaming_target_record::RoomStreamingTargetRecord;
pub use update_room_streaming_target::UpdateRoomStreamingTarget;
