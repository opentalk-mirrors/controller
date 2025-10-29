// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
};

use async_trait::async_trait;
use opentalk_r3dlock::Mutex;
use opentalk_signaling_core::{RedisConnection, RedisSnafu, SignalingModuleError, SignalingRoomId};
use opentalk_types_common::{
    rooms::RoomId,
    time::Timestamp,
    users::{GroupId, GroupName},
};
use opentalk_types_signaling::ParticipantId;
use opentalk_types_signaling_chat::state::{CHAT_CHUNK_SIZE, ChatChunk, StoredMessage};
use redis::AsyncCommands as _;
use redis_args::{FromRedisValue, ToRedisArgs};
use snafu::{OptionExt as _, Report, ResultExt as _};
use uuid::Uuid;

use super::ChatStorage;
use crate::{ParticipantPair, storage::room_private_chat_history::RoomPrivateChatHistory};

#[async_trait(?Send)]
impl ChatStorage for RedisConnection {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_room_history_chunk(
        &mut self,
        room: SignalingRoomId,
        message_index: u64,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let start = u64::saturating_sub(message_index, CHAT_CHUNK_SIZE - 1);
        let (messages, length) = redis::pipe()
            .atomic()
            .lrange(
                RoomChatHistory { room },
                start as isize,
                message_index as isize,
            )
            .llen(RoomChatHistory { room })
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get chat history chunk({start}, {message_index}): room={room}"
                ),
            })?;

        if message_index >= length {
            return Ok(ChatChunk::default());
        }

        Ok(ChatChunk {
            messages,
            next_index: start.checked_sub(1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_room_history_latest_chunk(
        &mut self,
        room: SignalingRoomId,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let (messages, length): (Vec<StoredMessage>, u64) = redis::pipe()
            .atomic()
            .lrange(RoomChatHistory { room }, -(CHAT_CHUNK_SIZE as isize), -1)
            .llen(RoomChatHistory { room })
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get chat history chunk(-{CHAT_CHUNK_SIZE}, -1): room={room}"
                ),
            })?;

        Ok(ChatChunk {
            messages,
            next_index: length.checked_sub(CHAT_CHUNK_SIZE + 1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn search_room_history(
        &mut self,
        room: SignalingRoomId,
        term: &str,
        message_index: Option<u64>,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let messages: Vec<StoredMessage> = self
            .lrange(RoomChatHistory { room }, 0, -1)
            .await
            .with_context(|_| RedisSnafu {
                message: format!("Failed to get room chat history: room={room}"),
            })?;
        Ok(search_history_chunked(messages, term, message_index))
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn add_message_to_room_history(
        &mut self,
        room: SignalingRoomId,
        message: &StoredMessage,
    ) -> Result<(), SignalingModuleError> {
        self.rpush(RoomChatHistory { room }, message)
            .await
            .with_context(|_| RedisSnafu {
                message: format!("Failed to add message to room chat history, room={room}"),
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_room_history(
        &mut self,
        room: SignalingRoomId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomChatHistory { room })
            .await
            .with_context(|_| RedisSnafu {
                message: format!("Failed to delete room chat history, room={room}"),
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn set_chat_enabled(
        &mut self,
        room: RoomId,
        enabled: bool,
    ) -> Result<(), SignalingModuleError> {
        self.set(ChatEnabled { room }, enabled)
            .await
            .context(RedisSnafu {
                message: "Failed to SET chat_enabled",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn is_chat_enabled(&mut self, room: RoomId) -> Result<bool, SignalingModuleError> {
        self.get(ChatEnabled { room })
            .await
            .context(RedisSnafu {
                message: "Failed to GET chat_enabled",
            })
            .map(|result: Option<bool>| result.unwrap_or(true))
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_chat_enabled(&mut self, room: RoomId) -> Result<(), SignalingModuleError> {
        self.del(ChatEnabled { room }).await.context(RedisSnafu {
            message: "Failed to DEL chat_enabled",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn set_last_seen_timestamps_private(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
        timestamps: &[(ParticipantId, Timestamp)],
    ) -> Result<(), SignalingModuleError> {
        self.hset_multiple(
            RoomParticipantLastSeenTimestampPrivate { room, participant },
            timestamps,
        )
        .await
        .context(RedisSnafu {
            message: "Failed to HSET messages last seen timestamp for private chat",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_last_seen_timestamps_private(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<BTreeMap<ParticipantId, Timestamp>, SignalingModuleError> {
        self.hgetall(RoomParticipantLastSeenTimestampPrivate { room, participant })
            .await
            .context(RedisSnafu {
                message: "Failed to HGETALL messages last seen timestamps for private chats",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_last_seen_timestamps_private(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomParticipantLastSeenTimestampPrivate { room, participant })
            .await
            .context(RedisSnafu {
                message: "Failed to DEL messages last seen timestamps for private chats",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn set_last_seen_timestamps_group(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
        timestamps: &[(GroupName, Timestamp)],
    ) -> Result<(), SignalingModuleError> {
        self.hset_multiple(
            RoomParticipantLastSeenTimestampsGroup { room, participant },
            timestamps,
        )
        .await
        .context(RedisSnafu {
            message: "Failed to HSET messages last seen timestamp for group chats",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_last_seen_timestamps_group(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<BTreeMap<GroupName, Timestamp>, SignalingModuleError> {
        self.hgetall(RoomParticipantLastSeenTimestampsGroup { room, participant })
            .await
            .context(RedisSnafu {
                message: "Failed to HGETALL messages last seen timestamp for group chats",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_last_seen_timestamps_group(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomParticipantLastSeenTimestampsGroup { room, participant })
            .await
            .context(RedisSnafu {
                message: "Failed to DEL last seen timestamp for group chats",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn set_last_seen_timestamp_global(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
        timestamp: Timestamp,
    ) -> Result<(), SignalingModuleError> {
        self.set(
            RoomParticipantLastSeenTimestampGlobal { room, participant },
            timestamp,
        )
        .await
        .context(RedisSnafu {
            message: "Failed to HSET messages last seen timestamp for global chat",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_last_seen_timestamp_global(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<Option<Timestamp>, SignalingModuleError> {
        let key = RoomParticipantLastSeenTimestampGlobal { room, participant };
        self.get(&key).await.context(RedisSnafu {
            message: "Failed to GET messages last seen timestamp for global chat",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_last_seen_timestamp_global(
        &mut self,
        room: SignalingRoomId,
        participant: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomParticipantLastSeenTimestampGlobal { room, participant })
            .await
            .context(RedisSnafu {
                message: "Failed to DEL messages last seen timestamp for global chat",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn add_private_chat_correspondents(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        let participants = ParticipantPair::new(participant_one, participant_two);
        self.sadd(
            RoomPrivateChatCorrespondentsKey { room },
            RoomPrivateChatCorrespondents::from(participants),
        )
        .await
        .context(RedisSnafu {
            message: "Failed to add private chat correspondents to set",
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_private_chat_correspondents(
        &mut self,
        room: SignalingRoomId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomPrivateChatCorrespondentsKey { room })
            .await
            .context(RedisSnafu {
                message: "Failed to delete private chat correspondents",
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_private_chat_correspondents(
        &mut self,
        room: SignalingRoomId,
    ) -> Result<HashSet<ParticipantPair>, SignalingModuleError> {
        let correspondents: HashSet<RoomPrivateChatCorrespondents> = self
            .smembers(RoomPrivateChatCorrespondentsKey { room })
            .await
            .context(RedisSnafu {
                message: "Failed to get private chat correspondents",
            })?;

        Ok(correspondents.into_iter().map(From::from).collect())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_group_chat_history_chunk(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
        message_index: u64,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let start = u64::saturating_sub(message_index, CHAT_CHUNK_SIZE - 1);
        let (messages, length) = redis::pipe()
            .atomic()
            .lrange(
                RoomGroupChatHistory { room, group },
                start as isize,
                message_index as isize,
            )
            .llen(RoomGroupChatHistory { room, group })
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get group chat history chunk({start}, {message_index}): room={room}, group={group}"
                ),
            })?;

        if message_index >= length {
            return Ok(ChatChunk::default());
        }

        Ok(ChatChunk {
            messages,
            next_index: start.checked_sub(1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_group_chat_history_latest_chunk(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let (messages, length): (Vec<StoredMessage>, u64) = redis::pipe()
            .atomic()
            .lrange(
                RoomGroupChatHistory { room, group },
                -(CHAT_CHUNK_SIZE as isize),
                -1,
            )
            .llen(RoomGroupChatHistory { room, group })
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get group chat history chunk(-{CHAT_CHUNK_SIZE}, -1): room={room}, group={group}"
                ),
            })?;

        Ok(ChatChunk {
            messages,
            next_index: length.checked_sub(CHAT_CHUNK_SIZE + 1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn search_group_chat_history(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
        term: &str,
        message_index: Option<u64>,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let messages: Vec<StoredMessage> = self
            .lrange(RoomGroupChatHistory { room, group }, 0, -1)
            .await
            .with_context(|_| RedisSnafu {
                message: format!("Failed to get group chat history: room={room}, group={group}"),
            })?;
        Ok(search_history_chunked(messages, term, message_index))
    }

    #[tracing::instrument(level = "debug", skip(self, message))]
    async fn add_message_to_group_chat_history(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
        message: &StoredMessage,
    ) -> Result<(), SignalingModuleError> {
        self.rpush(RoomGroupChatHistory { room, group }, message)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to add message to room chat history, {room}, group={group}",
                ),
            })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_group_chat_history(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
    ) -> Result<(), SignalingModuleError> {
        self
        .del(RoomGroupChatHistory { room, group })
        .await
        .with_context(|_| RedisSnafu {
            message: format!("Failed to delete room group chat history, {room}, group={group}",),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_private_chat_history_chunk(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
        message_index: u64,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let start = u64::saturating_sub(message_index, CHAT_CHUNK_SIZE - 1);

        let (messages, length) = redis::pipe()
            .atomic()
            .lrange(
                RoomPrivateChatHistory::new(room, participant_one, participant_two),
                start as isize,
                message_index as isize,
            )
            .llen(RoomPrivateChatHistory::new(
                room,
                participant_one,
                participant_two,
            ))
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get chat history chunk({start}, {message_index}): room={room}"
                ),
            })?;

        if message_index >= length {
            return Ok(ChatChunk::default());
        }

        Ok(ChatChunk {
            messages,
            next_index: start.checked_sub(1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_private_chat_history_latest_chunk(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let (messages, length): (Vec<StoredMessage>, u64) = redis::pipe()
            .atomic()
            .lrange(
                RoomPrivateChatHistory::new(
                    room,
                    participant_one,
                    participant_two,
                ),
                -(CHAT_CHUNK_SIZE as isize),
                -1,
            )
            .llen(RoomPrivateChatHistory::new(
                room,
                participant_one,
                participant_two,
            ))
            .query_async(self)
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get private chat history chunk(-{CHAT_CHUNK_SIZE}, -1): room={room}, participants={participant_one}, {participant_two}"
                ),
            })?;

        Ok(ChatChunk {
            messages,
            next_index: length.checked_sub(CHAT_CHUNK_SIZE + 1),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn search_private_chat_history(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
        term: &str,
        message_index: Option<u64>,
    ) -> Result<ChatChunk, SignalingModuleError> {
        let messages: Vec<StoredMessage> = self
            .lrange(
                RoomPrivateChatHistory::new(room, participant_one, participant_two),
                0,
                -1,
            )
            .await
            .with_context(|_| RedisSnafu {
                message: format!(
                    "Failed to get private chat history: \
                    room={room} \
                    participant_one={participant_one} \
                    participant_one={participant_two}"
                ),
            })?;
        Ok(search_history_chunked(messages, term, message_index))
    }

    #[tracing::instrument(level = "debug", skip(self, message))]
    async fn add_message_to_private_chat_history(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
        message: &StoredMessage,
    ) -> Result<(), SignalingModuleError> {
        self.rpush(
            RoomPrivateChatHistory::new(room, participant_one, participant_two),
            message,
        )
        .await
        .with_context(|_| RedisSnafu {
            message: format!(
                "Failed to add message to room private chat history, {room}, \
                participants {participant_one} and {participant_two}",
            ),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_private_chat_history(
        &mut self,
        room: SignalingRoomId,
        participant_one: ParticipantId,
        participant_two: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        self.del(RoomPrivateChatHistory::new(
            room,
            participant_one,
            participant_two,
        ))
        .await
        .with_context(|_| RedisSnafu {
            message: format!(
                "Failed to delete room private chat history, {room}, \
                participants {participant_one} and {participant_two}"
            ),
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn add_participant_to_group(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
        participant: ParticipantId,
    ) -> Result<(), SignalingModuleError> {
        let mutex = Mutex::new(RoomGroupParticipantsLock { room, group });

        let guard = mutex.lock(self).await?;

        self.sadd::<_, _, ()>(RoomGroupParticipants { room, group }, participant)
            .await
            .context(RedisSnafu {
                message: "Failed to add own participant id to set",
            })?;

        guard.unlock(self).await?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn remove_participant_from_group(
        &mut self,
        room: SignalingRoomId,
        group: GroupId,
        participant: ParticipantId,
    ) {
        let mutex = Mutex::new(RoomGroupParticipantsLock { room, group });

        let guard = match mutex.lock(self).await {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("Failed to acquire lock to cleanup group {group:?}, {e}");
                return;
            }
        };

        if let Err(e) = self
            .srem::<_, _, ()>(RoomGroupParticipants { room, group }, participant)
            .await
        {
            log::error!("Failed to remove participant from group {group:?}, {e}");
        };

        let remove_history = match self
            .scard::<_, usize>(RoomGroupParticipants { room, group })
            .await
            .context(RedisSnafu {
                message: "Failed to get number of remaining participants inside the set",
            }) {
            Ok(n) => n == 0,
            Err(e) => {
                log::error!(
                    "Failed to remove participant from group set, {}",
                    Report::from_error(e)
                );
                false
            }
        };

        if remove_history && let Err(e) = self.delete_group_chat_history(room, group).await {
            log::error!(
                "Failed to remove room group chat history, {}",
                Report::from_error(e)
            );
        };

        if let Err(e) = guard.unlock(self).await {
            log::error!("Failed to unlock r3dlock, {}", Report::from_error(e));
        }
    }
}

fn search_history_chunked(
    messages: Vec<StoredMessage>,
    term: &str,
    message_index: Option<u64>,
) -> ChatChunk {
    if term.is_empty() {
        return ChatChunk::default();
    }

    // Filter the messages by search term
    let filtered: Vec<StoredMessage> = messages
        .into_iter()
        .filter(|m| m.content.contains(term))
        .collect();

    // When the message_index is None, the latest chunk should be returned, set it
    // to the last index.
    let message_index = message_index
        .map(|i| i as usize)
        .unwrap_or(filtered.len().saturating_sub(1));
    let start = message_index.saturating_sub(CHAT_CHUNK_SIZE as usize - 1);
    // A chunk might be smaller than the chunk size, when it is less than a
    // chunk size away from the end of the list.
    let messages: Vec<StoredMessage> = filtered
        .get(start..=message_index)
        .unwrap_or_default()
        .iter()
        .map(|message| (*message).clone())
        .collect();

    // When no messages could be found the message_index was OOB or no messages
    // exist. The next_index should be None in this case.
    let next_index = if messages.is_empty() {
        None
    } else {
        (start as u64).checked_sub(1)
    };

    ChatChunk {
        messages,
        next_index,
    }
}

/// Key to the chat history inside a room
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:chat:history")]
struct RoomChatHistory {
    room: SignalingRoomId,
}

/// If set to true the chat is enabled
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:chat_enabled")]
struct ChatEnabled {
    room: RoomId,
}

/// A hash of last-seen timestamps
#[derive(ToRedisArgs)]
#[to_redis_args(
    fmt = "opentalk-signaling:room={room}:participant={participant}:chat:last_seen:private"
)]
struct RoomParticipantLastSeenTimestampPrivate {
    room: SignalingRoomId,
    participant: ParticipantId,
}

/// A hash of last-seen timestamps
#[derive(ToRedisArgs)]
#[to_redis_args(
    fmt = "opentalk-signaling:room={room}:participant={participant}:chat:last_seen:group"
)]
struct RoomParticipantLastSeenTimestampsGroup {
    room: SignalingRoomId,
    participant: ParticipantId,
}

/// A hash of last-seen timestamps
#[derive(ToRedisArgs)]
#[to_redis_args(
    fmt = "opentalk-signaling:room={room}:participant={participant}:chat:last_seen:global"
)]
struct RoomParticipantLastSeenTimestampGlobal {
    room: SignalingRoomId,
    participant: ParticipantId,
}

/// A set of private chat correspondents for a participant in a room
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:private_chat_correspondents")]
struct RoomPrivateChatCorrespondentsKey {
    room: SignalingRoomId,
}

#[derive(ToRedisArgs, FromRedisValue, Eq, PartialEq, Debug, Hash)]
#[to_redis_args(fmt = "{participant_one}:{participant_two}")]
#[from_redis_value(FromStr)]
struct RoomPrivateChatCorrespondents {
    participant_one: ParticipantId,
    participant_two: ParticipantId,
}

impl From<ParticipantPair> for RoomPrivateChatCorrespondents {
    fn from(value: ParticipantPair) -> Self {
        Self {
            participant_one: value.participant_one(),
            participant_two: value.participant_two(),
        }
    }
}

impl From<RoomPrivateChatCorrespondents> for ParticipantPair {
    fn from(value: RoomPrivateChatCorrespondents) -> Self {
        Self::new(value.participant_one, value.participant_two)
    }
}

impl FromStr for RoomPrivateChatCorrespondents {
    type Err = SignalingModuleError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let participants = s
            .split_once(':')
            .whatever_context::<&str, SignalingModuleError>(
                "Failed to split RoomPrivateChatCorrespondents",
            )?;

        Ok(Self {
            participant_one: ParticipantId::from(Uuid::from_str(participants.0)?),
            participant_two: ParticipantId::from(Uuid::from_str(participants.1)?),
        })
    }
}

/// Chat history for a group inside a room
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:group={group}:chat:history")]
struct RoomGroupChatHistory {
    room: SignalingRoomId,
    group: GroupId,
}

/// A set of group members inside a room
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:group={group}:participants")]
struct RoomGroupParticipants {
    room: SignalingRoomId,
    group: GroupId,
}

/// A lock for the set of group members inside a room
#[derive(ToRedisArgs)]
#[to_redis_args(fmt = "opentalk-signaling:room={room}:group={group}:participants.lock")]
pub struct RoomGroupParticipantsLock {
    pub room: SignalingRoomId,
    pub group: GroupId,
}

#[cfg(test)]
mod tests {
    use redis::{ToRedisArgs, aio::ConnectionManager};
    use serial_test::serial;
    use uuid::uuid;

    use super::{super::test_common, *};

    async fn storage() -> RedisConnection {
        let redis_url =
            std::env::var("REDIS_ADDR").unwrap_or_else(|_| "redis://0.0.0.0:6379/".to_owned());
        let redis = redis::Client::open(redis_url).expect("Invalid redis url");

        let mut mgr = ConnectionManager::new(redis).await.unwrap();

        redis::cmd("FLUSHALL").exec_async(&mut mgr).await.unwrap();

        RedisConnection::new(mgr)
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_last_seen_global() {
        test_common::last_seen_global(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_last_seen_global_is_personal() {
        test_common::last_seen_global_is_personal(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_last_seen_private() {
        test_common::last_seen_private(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_last_seen_private_is_personal() {
        test_common::last_seen_private_is_personal(&mut storage().await).await;
    }

    #[test]
    fn redis_args() {
        let room_id = RoomId::from(uuid!("ecead1b3-eed0-4cb9-912e-4bb31a3914bd"));

        {
            let id = RoomChatHistory {
                room: SignalingRoomId::new_for_room(room_id),
            };
            assert_eq!(
                id.to_redis_args(),
                "opentalk-signaling:room=ecead1b3-eed0-4cb9-912e-4bb31a3914bd:chat:history"
                    .to_redis_args()
            );
        }
        {
            let id = ChatEnabled { room: room_id };
            assert_eq!(
                id.to_redis_args(),
                "opentalk-signaling:room=ecead1b3-eed0-4cb9-912e-4bb31a3914bd:chat_enabled"
                    .to_redis_args()
            )
        }
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_room_chat_history() {
        test_common::room_chat_history(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_group_chat_history() {
        test_common::group_chat_history(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_private_chat_history() {
        test_common::private_chat_history(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_search_room_chat_history() {
        test_common::search_room_chat_history(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_search_group_chat_history() {
        test_common::search_group_chat_history(&mut storage().await).await;
    }

    #[tokio::test]
    #[serial]
    async fn serial_test_search_private_chat_history() {
        test_common::search_private_chat_history(&mut storage().await).await;
    }
}
