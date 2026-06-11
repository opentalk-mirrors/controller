// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Authorization handlers for event-related resources.

mod email_invite;
#[allow(clippy::module_inception)]
mod event;
mod events;
mod events_instances;
mod instance;
mod instances;
mod invite;
mod invites;
mod shared_folder;
mod user_invite;

#[cfg(test)]
pub(crate) mod test_utils {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    use mockall::predicate::eq;
    use opentalk_controller_settings::settings_provider_from_example_raw_settings;
    use opentalk_inventory::{
        AuthorizationInventory, AuthorizationInviteCodeValidity as Validity,
        AuthorizationUserRole as Role, MockAuthorizationInventory, MockInventoryProvider,
    };
    use opentalk_types_common::{
        events::EventId,
        features::{FeatureId, ModuleFeatureId},
        modules::ModuleId,
        rooms::invite_codes::InviteCode,
        users::UserId,
    };

    use crate::OpenTalkAuthorizerBackend;

    pub const EVENT_ID: EventId = EventId::from_u128(0x0001);
    pub const USER_ID: UserId = UserId::from_u128(0x0002);
    pub const INVITE_CODE: InviteCode = InviteCode::from_u128(0x0003);
    pub const DISABLED_FEATURES: BTreeSet<ModuleFeatureId> = BTreeSet::new();
    pub const MODULE_FEATURES: BTreeMap<ModuleId, BTreeSet<FeatureId>> = BTreeMap::new();

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
            .expect_get_event_user_role()
            .with(eq(EVENT_ID), eq(USER_ID))
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
            .expect_get_event_invite_code_validity()
            .with(
                eq(EVENT_ID),
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
