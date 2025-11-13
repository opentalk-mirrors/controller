// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_signaling_core::module_tester::{ModuleTester, WsMessageOutgoing};
use opentalk_test_util::{ROOM_ID, TestContext, USER_1, USER_2};
use opentalk_types_signaling::Role;
use opentalk_types_signaling_asset_storage::state::AssetStorageState;
use opentalk_types_signaling_control::event::ControlEvent;
use serial_test::serial;

#[actix_rt::test]
#[serial]
async fn serial_test_room_without_event() {
    let test_ctx = TestContext::default().await;

    let user1 = test_ctx
        .db_ctx
        .create_test_user(USER_1.n, vec![])
        .await
        .unwrap();

    let user2 = test_ctx
        .db_ctx
        .create_test_user(USER_2.n, vec![])
        .await
        .unwrap();

    let waiting_room = false;
    let room = test_ctx
        .db_ctx
        .create_test_room(ROOM_ID, user1.id, waiting_room)
        .await
        .unwrap();

    let mut module_tester =
        ModuleTester::<opentalk_signaling_module_asset_storage::AssetStorage>::new(
            test_ctx.db_ctx.inventory_provider.clone(),
            test_ctx.authz,
            test_ctx.volatile,
            room,
        );

    {
        // join a moderator user
        module_tester
            .join_user(
                USER_1.participant_id,
                user1.clone(),
                Role::Moderator,
                &USER_1.display_name(),
                (),
            )
            .await
            .unwrap();
        let join_success = module_tester
            .receive_ws_message(&USER_1.participant_id)
            .await
            .unwrap();
        match join_success {
            WsMessageOutgoing::Control(ControlEvent::JoinSuccess(join_success)) => {
                // check that no asset store information is available
                join_success
                    .module_data
                    .get::<AssetStorageState>()
                    .expect("Missing asset_storage module")
                    .expect("Missing asset_storage module data");
            }
            _ => panic!("Expected join success"),
        }
    }

    {
        // join a non-moderator user
        module_tester
            .join_user(
                USER_2.participant_id,
                user2.clone(),
                Role::User,
                &USER_2.display_name(),
                (),
            )
            .await
            .unwrap();
        let join_success = module_tester
            .receive_ws_message(&USER_2.participant_id)
            .await
            .unwrap();
        match join_success {
            WsMessageOutgoing::Control(ControlEvent::JoinSuccess(join_success)) => {
                // check that no asset store information is available
                let asset_module_state = join_success
                    .module_data
                    .get::<AssetStorageState>()
                    .expect("Missing asset_storage module");

                assert!(
                    asset_module_state.is_none(),
                    "non-moderators must not receive asset storage information"
                )
            }
            _ => panic!(),
        }
    }
}
