// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Authorization handlers for room-related resources.

mod asset;
mod asset_download;
mod assets;
mod event;
mod invite_code;
mod invites;
#[allow(clippy::module_inception)]
mod room;
mod rooms;
mod sip;
mod start;
mod streaming_target;
mod streaming_targets;
mod tariff;

#[cfg(test)]
pub(crate) mod test_utils {
    use std::sync::Arc;

    use mockall::predicate::eq;
    use opentalk_controller_settings::settings_provider_from_example_raw_settings;
    use opentalk_inventory::{
        AuthorizationInventory, AuthorizationInviteCodeValidity as Validity,
        AuthorizationUserRole as Role, MockAuthorizationInventory, MockInventoryProvider,
    };
    use opentalk_types_common::{
        rooms::{RoomId, invite_codes::InviteCode},
        users::UserId,
    };

    pub const ROOM_ID: RoomId = RoomId::from_u128(0x0001);
    pub const USER_ID: UserId = UserId::from_u128(0x0002);
    pub const INVITE_CODE: InviteCode = InviteCode::from_u128(0x0003);

    use crate::{
        OpenTalkAuthorizerBackend,
        event::test_utils::{DISABLED_FEATURES, MODULE_FEATURES},
    };

    pub fn create_authorizer_with_role(role: Role) -> OpenTalkAuthorizerBackend {
        let inventory_provider = get_mock_inventory_provider_returning_role(role);
        OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            settings_provider_from_example_raw_settings(),
            MODULE_FEATURES,
        )
    }

    pub fn create_authorizer_with_validity(validity: Validity) -> OpenTalkAuthorizerBackend {
        let inventory_provider = get_mock_inventory_provider_returning_validity(validity);
        OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            settings_provider_from_example_raw_settings(),
            MODULE_FEATURES,
        )
    }

    pub fn get_mock_inventory_provider_returning_role(role: Role) -> MockInventoryProvider {
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_room_user_role()
            .with(eq(ROOM_ID), eq(USER_ID))
            .return_once(move |_, _| Ok(role));
        let inventory: Box<dyn AuthorizationInventory> = Box::new(inventory);

        let mut inventory_provider = MockInventoryProvider::new();
        let _ = inventory_provider
            .expect_get_authorization_inventory()
            .return_once(move || Ok(inventory));

        inventory_provider
    }

    pub fn get_mock_inventory_provider_returning_validity(
        validity: Validity,
    ) -> MockInventoryProvider {
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_room_invite_code_validity()
            .with(
                eq(ROOM_ID),
                eq(INVITE_CODE),
                eq(DISABLED_FEATURES),
                eq(MODULE_FEATURES),
            )
            .return_once(move |_, _, _, _| Ok(validity));
        let inventory: Box<dyn AuthorizationInventory> = Box::new(inventory);

        let mut inventory_provider = MockInventoryProvider::new();
        let _ = inventory_provider
            .expect_get_authorization_inventory()
            .return_once(move || Ok(inventory));

        inventory_provider
    }
}
