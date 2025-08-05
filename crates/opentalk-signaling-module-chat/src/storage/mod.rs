// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

mod chat_storage;
mod redis;
mod room_private_chat_history;
mod volatile;

pub(crate) use chat_storage::ChatStorage;

#[cfg(test)]
mod test_common {
    use std::{
        collections::BTreeMap,
        str::FromStr,
        time::{Duration, SystemTime},
    };

    use chrono::{DateTime, Utc};
    use opentalk_signaling_core::SignalingRoomId;
    use opentalk_types_common::{
        rooms::RoomId,
        time::Timestamp,
        users::{GroupId, GroupName},
    };
    use opentalk_types_signaling::ParticipantId;
    use opentalk_types_signaling_chat::{
        MessageId, Scope,
        state::{CHAT_CHUNK_SIZE, ChatChunk, StoredMessage},
    };
    use pretty_assertions::assert_eq;

    use super::*;

    pub const ROOM: SignalingRoomId = SignalingRoomId::nil();
    pub const SELF: ParticipantId = ParticipantId::nil();
    pub const BOB: ParticipantId = ParticipantId::from_u128(0xdeadbeef);
    pub const ALICE: ParticipantId = ParticipantId::from_u128(0xbadcafe);

    fn unix_epoch(secs: u64) -> DateTime<Utc> {
        DateTime::from(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
    }

    pub(super) async fn last_seen_global(storage: &mut dyn ChatStorage) {
        assert!(
            storage
                .get_last_seen_timestamp_global(ROOM, SELF)
                .await
                .unwrap()
                .is_none()
        );

        storage
            .set_last_seen_timestamp_global(ROOM, SELF, unix_epoch(1000).into())
            .await
            .unwrap();

        assert_eq!(
            storage
                .get_last_seen_timestamp_global(ROOM, SELF)
                .await
                .unwrap(),
            Some(unix_epoch(1000).into())
        );

        storage
            .delete_last_seen_timestamp_global(ROOM, SELF)
            .await
            .unwrap();

        assert!(
            storage
                .get_last_seen_timestamp_global(ROOM, SELF)
                .await
                .unwrap()
                .is_none()
        );
    }

    pub(super) async fn last_seen_global_is_personal(storage: &mut dyn ChatStorage) {
        // Set the private last seen timestamps as if BOB and ALICE were the participants in the
        // same room, and ensure this doesn't affect the timestamps of SELF.
        {
            // Set BOB's timestamp
            storage
                .set_last_seen_timestamp_global(ROOM, BOB, unix_epoch(1000).into())
                .await
                .unwrap();
        }
        {
            // Set ALICE's timestamp
            storage
                .set_last_seen_timestamp_global(ROOM, ALICE, unix_epoch(2000).into())
                .await
                .unwrap();
        }

        assert!(
            storage
                .get_last_seen_timestamp_global(ROOM, SELF)
                .await
                .unwrap()
                .is_none()
        );
    }

    pub(super) async fn last_seen_private(storage: &mut dyn ChatStorage) {
        assert!(
            storage
                .get_last_seen_timestamps_private(ROOM, SELF)
                .await
                .unwrap()
                .is_empty(),
        );

        storage
            .set_last_seen_timestamps_private(ROOM, SELF, &[(BOB, unix_epoch(1000).into())])
            .await
            .unwrap();

        assert_eq!(
            storage
                .get_last_seen_timestamps_private(ROOM, SELF)
                .await
                .unwrap(),
            BTreeMap::from_iter([(BOB, unix_epoch(1000).into())])
        );

        storage
            .set_last_seen_timestamps_private(ROOM, SELF, &[(ALICE, unix_epoch(2000).into())])
            .await
            .unwrap();

        assert_eq!(
            storage
                .get_last_seen_timestamps_private(ROOM, SELF)
                .await
                .unwrap(),
            BTreeMap::from_iter([
                (BOB, unix_epoch(1000).into()),
                (ALICE, unix_epoch(2000).into()),
            ])
        );

        storage
            .delete_last_seen_timestamps_private(ROOM, SELF)
            .await
            .unwrap();

        assert!(
            storage
                .get_last_seen_timestamps_private(ROOM, SELF)
                .await
                .unwrap()
                .is_empty(),
        );
    }

    pub(super) async fn last_seen_private_is_personal(storage: &mut dyn ChatStorage) {
        // Set the private last seen timestamps as if BOB and ALICE were the participants in the
        // same room, and ensure this doesn't affect the timestamps of SELF.
        {
            // Set BOB's personal timestamps
            storage
                .set_last_seen_timestamps_private(
                    ROOM,
                    BOB,
                    &[
                        (ALICE, unix_epoch(1000).into()),
                        (SELF, unix_epoch(2000).into()),
                    ],
                )
                .await
                .unwrap();
        }
        {
            // Set ALICE's personal timestamps
            storage
                .set_last_seen_timestamps_private(ROOM, ALICE, &[(SELF, unix_epoch(3000).into())])
                .await
                .unwrap();
        }

        assert!(
            storage
                .get_last_seen_timestamps_private(ROOM, SELF)
                .await
                .unwrap()
                .is_empty()
        );
    }

    pub(super) async fn room_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());

        let chunk = storage.get_room_history_latest_chunk(room).await.unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_room_messages(storage, room, "", message_count).await;

        let chunk = storage.get_room_history_latest_chunk(room).await.unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .get_room_history_chunk(room, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .get_room_history_chunk(room, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .get_room_history_chunk(room, CHAT_CHUNK_SIZE * 100)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }

    async fn fill_room_messages(
        storage: &mut dyn ChatStorage,
        room: SignalingRoomId,
        message_content: &str,
        message_count: u64,
    ) {
        for i in 0..message_count {
            storage
                .add_message_to_room_history(
                    room,
                    &StoredMessage {
                        id: MessageId::generate(),
                        source: ParticipantId::generate(),
                        timestamp: Timestamp::now(),
                        content: format!("{i}_{message_content}"),
                        scope: Scope::Global,
                    },
                )
                .await
                .unwrap();
        }
    }

    pub(super) async fn group_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());
        let group = GroupId::generate();

        let chunk = storage
            .get_group_chat_history_latest_chunk(room, group)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_group_history(storage, room, group, "", message_count).await;

        let chunk = storage
            .get_group_chat_history_latest_chunk(room, group)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .get_group_chat_history_chunk(room, group, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .get_group_chat_history_chunk(room, group, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .get_group_chat_history_chunk(room, group, CHAT_CHUNK_SIZE * 100)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }

    async fn fill_group_history(
        storage: &mut dyn ChatStorage,
        room: SignalingRoomId,
        group: GroupId,
        message_content: &str,
        message_count: u64,
    ) {
        for i in 0..message_count {
            storage
                .add_message_to_group_chat_history(
                    room,
                    group,
                    &StoredMessage {
                        id: MessageId::generate(),
                        source: ParticipantId::generate(),
                        timestamp: Timestamp::now(),
                        content: format!("{i}_{message_content}"),
                        scope: Scope::Group(GroupName::from_str("group1").unwrap()),
                    },
                )
                .await
                .unwrap();
        }
    }

    pub(super) async fn private_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());
        let alice = ParticipantId::generate();
        let bob = ParticipantId::generate();

        let chunk = storage
            .get_private_chat_history_latest_chunk(room, alice, bob)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_private_messages(storage, room, alice, bob, "", message_count).await;

        let chunk = storage
            .get_private_chat_history_latest_chunk(room, alice, bob)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .get_private_chat_history_chunk(room, alice, bob, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .get_private_chat_history_chunk(room, alice, bob, chunk.next_index.unwrap())
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .get_private_chat_history_chunk(room, alice, bob, CHAT_CHUNK_SIZE * 100)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }

    async fn fill_private_messages(
        storage: &mut dyn ChatStorage,
        room: SignalingRoomId,
        alice: ParticipantId,
        bob: ParticipantId,
        message_content: &str,
        message_count: u64,
    ) {
        for i in 0..message_count {
            storage
                .add_message_to_private_chat_history(
                    room,
                    alice,
                    bob,
                    &StoredMessage {
                        id: MessageId::generate(),
                        source: alice,
                        timestamp: Timestamp::now(),
                        content: format!("{i}_{message_content}"),
                        scope: Scope::Private(bob),
                    },
                )
                .await
                .unwrap();
        }
    }

    pub(super) async fn search_room_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());

        let chunk = storage
            .search_room_history(room, "test", None)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_room_messages(storage, room, "hello", message_count).await;
        fill_room_messages(storage, room, "goodbye", message_count).await;

        let chunk = storage
            .search_room_history(room, "hello", None)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .search_room_history(room, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .search_room_history(room, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .search_room_history(room, "hello", Some(CHAT_CHUNK_SIZE * 1000))
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }

    pub(super) async fn search_group_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());
        let group = GroupId::generate();

        let chunk = storage
            .search_group_chat_history(room, group, "hello", None)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_group_history(storage, room, group, "hello", message_count).await;
        fill_group_history(storage, room, group, "goodbye", message_count).await;

        let chunk = storage
            .search_group_chat_history(room, group, "hello", None)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .search_group_chat_history(room, group, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .search_group_chat_history(room, group, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .search_group_chat_history(room, group, "hello", Some(CHAT_CHUNK_SIZE * 1000))
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }

    pub(super) async fn search_private_chat_history(storage: &mut dyn ChatStorage) {
        let room = SignalingRoomId::new_for_room(RoomId::generate());
        let alice = ParticipantId::generate();
        let bob = ParticipantId::generate();

        let chunk = storage
            .search_private_chat_history(room, alice, bob, "hello", None)
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());

        let message_count = (2 * CHAT_CHUNK_SIZE) + 1;
        fill_private_messages(storage, room, alice, bob, "hello", message_count).await;
        fill_private_messages(storage, room, alice, bob, "goodbye", message_count).await;

        let chunk = storage
            .search_private_chat_history(room, alice, bob, "hello", None)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(message_count - CHAT_CHUNK_SIZE - 1));

        // Check for the correct order
        assert_eq!(
            chunk
                .messages
                .iter()
                .position(|m| m.content.contains(&(message_count - 1).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        // Getting chunks should not be affected by new messages being added
        storage
            .add_message_to_room_history(
                room,
                &StoredMessage {
                    id: MessageId::generate(),
                    source: ParticipantId::generate(),
                    timestamp: Timestamp::now(),
                    content: "x_hello".into(),
                    scope: Scope::Global,
                },
            )
            .await
            .unwrap();

        let chunk = storage
            .search_private_chat_history(room, alice, bob, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len() as u64, CHAT_CHUNK_SIZE);
        assert_eq!(chunk.next_index, Some(0));

        // Check for the correct order
        assert_eq!(
            chunk.messages.iter().position(|m| m
                .content
                .contains(&(message_count - 1 - CHAT_CHUNK_SIZE).to_string())),
            Some(CHAT_CHUNK_SIZE as usize - 1)
        );

        let chunk = storage
            .search_private_chat_history(room, alice, bob, "hello", chunk.next_index)
            .await
            .unwrap();

        assert_eq!(chunk.messages.len(), 1);
        assert_eq!(chunk.next_index, None);

        // Out of bounds
        let chunk = storage
            .search_private_chat_history(room, alice, bob, "hello", Some(CHAT_CHUNK_SIZE * 1000))
            .await
            .unwrap();

        assert_eq!(chunk, ChatChunk::default());
    }
}
