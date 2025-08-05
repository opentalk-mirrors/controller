// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_signaling_core::SignalingRoomId;
use opentalk_types_signaling::ParticipantId;
use redis_args::ToRedisArgs;

use crate::participant_pair::ParticipantPair;

/// Private chat history for two participants inside a room
#[derive(ToRedisArgs)]
#[to_redis_args(
    fmt = "opentalk-signaling:room={room}:participant={participant_one}:participant={participant_two}:chat:history"
)]
pub struct RoomPrivateChatHistory {
    room: SignalingRoomId,
    participant_one: ParticipantId,
    participant_two: ParticipantId,
}

impl RoomPrivateChatHistory {
    pub fn new(
        room: SignalingRoomId,
        participant_a: ParticipantId,
        participant_b: ParticipantId,
    ) -> Self {
        let pair = ParticipantPair::new(participant_a, participant_b);
        Self {
            room,
            participant_one: pair.participant_one(),
            participant_two: pair.participant_two(),
        }
    }
}
