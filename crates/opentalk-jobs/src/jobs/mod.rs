// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! # Jobs that can be run in the OpenTalk job execution system

mod adhoc_event_cleanup;
mod event_cleanup;
mod invite_cleanup;
mod keycloak_account_sync;
mod room_cleanup;
mod self_check;
mod sync_storage_files;
mod user_cleanup;

pub use adhoc_event_cleanup::AdhocEventCleanup;
pub use event_cleanup::EventCleanup;
pub use invite_cleanup::InviteCleanup;
pub use keycloak_account_sync::KeycloakAccountSync;
pub use room_cleanup::RoomCleanup;
pub use self_check::SelfCheck;
pub use sync_storage_files::SyncStorageFiles;
pub use user_cleanup::UserCleanup;

#[cfg(test)]
mod test_utils {
    use opentalk_inventory::{
        Event, Inventory, InventoryProvider as _, NewEvent, NewRoom, NewRoomInvite, Room,
        RoomInvite, User,
    };
    use opentalk_test_util::database::DatabaseContext;

    pub(super) async fn create_events_and_independent_rooms(
        db_ctx: &DatabaseContext,
        event_count: u64,
        independent_room_count: u64,
    ) {
        let user = db_ctx.create_test_user(1, vec![]).await.unwrap();

        let mut inventory = db_ctx.inventory_provider.get_inventory().await.unwrap();

        for _ in 0..event_count {
            create_generic_test_event(inventory.as_mut(), &user).await;
        }

        for _ in 0..independent_room_count {
            create_generic_test_room(inventory.as_mut(), &user).await;
        }
    }

    pub(super) async fn create_generic_test_room(
        inventory: &mut dyn Inventory,
        user: &User,
    ) -> Room {
        inventory
            .create_room(NewRoom {
                created_by: user.id,
                password: None,
                waiting_room: false,
                e2e_encryption: false,
                tenant_id: user.tenant_id,
            })
            .await
            .unwrap()
    }

    pub(super) async fn create_generic_test_event(
        inventory: &mut dyn Inventory,
        user: &User,
    ) -> Event {
        let room = create_generic_test_room(inventory, user).await;

        inventory
            .create_event(NewEvent {
                title: "TestEvent".parse().expect("valid event title"),
                description: "A normal event, created by a test"
                    .parse()
                    .expect("valid event description"),
                room: room.id,
                created_by: user.id,
                updated_by: user.id,
                date: None,
                is_adhoc: true,
                tenant_id: user.tenant_id,
                show_meeting_details: true,
            })
            .await
            .unwrap()
    }

    pub(super) async fn create_generic_test_invite(
        inventory: &mut dyn Inventory,
        inviter: &User,
        updated_by: Option<&User>,
        room: &Room,
    ) -> RoomInvite {
        inventory
            .create_room_invite(NewRoomInvite {
                created_by: inviter.id,
                updated_by: updated_by.unwrap_or(inviter).id,
                room: room.id,
                active: true,
                expiration: None,
            })
            .await
            .unwrap()
    }
}
