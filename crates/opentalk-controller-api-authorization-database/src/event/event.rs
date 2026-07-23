// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, SubjectCollection,
};
use opentalk_types_common::events::EventId;

use crate::{
    OpenTalkAuthorizerBackend, Result,
    common::acl::{Access, Acl},
};

impl OpenTalkAuthorizerBackend {
    /// Authorize access to an [`Event`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`Event`] resource.
    ///
    /// ```text
    /// | Subject                 | Access |
    /// | ----------------------- | ------ |
    /// | **Owner**               | rw     |
    /// | **Moderator**           | r-     |
    /// | **Invited-User**        | r-     |
    /// | **Unrelated-User**      | --     |
    /// | **Valid Invite-Code**   | --     |
    /// | **Invalid Invite-Code** | --     |
    /// ```
    ///
    /// [`Event`]: opentalk_controller_api_authorization::authorization::Resource::Event
    pub(crate) async fn authorize_event(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::Read,
            invited_user: Access::Read,
            invite_code: Access::None,
        };

        self.apply_acl_for_event(subjects, method, event_id, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mockall::predicate::eq;
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Get as GET, Post as POST},
        Admission::{self, Allowed, Denied},
        AuthorizationTarget, AuthorizerBackend, Resource, Subject, SubjectCollection,
    };
    use opentalk_controller_settings::test_util;
    use opentalk_inventory::{
        AuthorizationInventory,
        AuthorizationInviteCodeValidity::{self, Invalid, Valid},
        AuthorizationUserRole::{self, Invited, Owner, Unrelated},
        MockAuthorizationInventory, MockInventoryProvider,
    };
    use opentalk_types_common::events::invites::InviteRole::{Moderator, User};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::super::test_utils::{
        EVENT_ID, INVITE_CODE, USER_ID, create_authorizer_with_role,
        create_authorizer_with_validity,
    };
    use crate::{
        OpenTalkAuthorizerBackend,
        event::test_utils::{DISABLED_FEATURES, MODULE_FEATURES},
    };

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, GET, Allowed)]
    #[case::owner_post(Owner, POST, Allowed)]
    #[case::moderator_get(Invited(Moderator), GET, Allowed)]
    #[case::moderator_post(Invited(Moderator), POST, Denied)]
    #[case::user_get(Invited(User), GET, Allowed)]
    #[case::user_post(Invited(User), POST, Denied)]
    #[case::unrelated_get(Unrelated, GET, Denied)]
    #[case::unrelated_post(Unrelated, POST, Denied)]
    async fn user(
        #[case] role: AuthorizationUserRole,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_role(role);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(USER_ID)]),
                resource: Resource::Event(EVENT_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, GET, Denied)]
    #[case::valid_post(Valid, POST, Denied)]
    #[case::invalid_get(Invalid, GET, Denied)]
    #[case::invalid_post(Invalid, POST, Denied)]
    async fn invite_code(
        #[case] validity: AuthorizationInviteCodeValidity,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_validity(validity);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(INVITE_CODE)]),
                resource: Resource::Event(EVENT_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    async fn multiple_subjects_first_allowed_wins() {
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_event_user_role()
            .with(eq(EVENT_ID), eq(USER_ID))
            .return_once(move |_, _| Ok(Invited(User)));
        let _ = inventory
            .expect_get_event_invite_code_validity()
            .with(
                eq(EVENT_ID),
                eq(INVITE_CODE),
                eq(DISABLED_FEATURES),
                eq(MODULE_FEATURES),
            )
            .return_once(move |_, _, _, _| Ok(Valid));
        let inventory: Box<dyn AuthorizationInventory> = Box::new(inventory);

        let mut inventory_provider = MockInventoryProvider::new();
        let _ = inventory_provider
            .expect_get_authorization_inventory()
            .return_once(move || Ok(inventory));

        let authorizer = OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            test_util::settings_provider_from_example_raw_settings(),
            MODULE_FEATURES,
        );
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([
                    Subject::from(INVITE_CODE),
                    Subject::from(USER_ID),
                ]),
                resource: Resource::Event(EVENT_ID),
                access_method: AccessMethod::Get,
            })
            .await
            .unwrap();
        assert_eq!(Allowed, admission);
    }

    #[tokio::test]
    async fn multiple_subjects_all_denied() {
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_event_user_role()
            .with(eq(EVENT_ID), eq(USER_ID))
            .return_once(move |_, _| Ok(Unrelated));
        let _ = inventory
            .expect_get_event_invite_code_validity()
            .with(
                eq(EVENT_ID),
                eq(INVITE_CODE),
                eq(DISABLED_FEATURES),
                eq(MODULE_FEATURES),
            )
            .return_once(move |_, _, _, _| Ok(Invalid));
        let inventory: Box<dyn AuthorizationInventory> = Box::new(inventory);

        let mut inventory_provider = MockInventoryProvider::new();
        let _ = inventory_provider
            .expect_get_authorization_inventory()
            .return_once(move || Ok(inventory));

        let authorizer = OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            test_util::settings_provider_from_example_raw_settings(),
            MODULE_FEATURES,
        );
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([
                    Subject::from(USER_ID),
                    Subject::from(INVITE_CODE),
                ]),
                resource: Resource::Event(EVENT_ID),
                access_method: AccessMethod::Get,
            })
            .await
            .unwrap();
        assert_eq!(Denied, admission);
    }
}
