// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, SubjectCollection,
};
use opentalk_types_common::rooms::RoomId;

use crate::{
    OpenTalkAuthorizerBackend, Result,
    common::acl::{Access, Acl},
};

impl OpenTalkAuthorizerBackend {
    /// Authorize access to a [`Room`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`Room`] resource (`GET`, `PATCH`,
    /// `DELETE` on `/rooms/{room_id}`). Modifying or deleting a room
    /// is owner-only by design: the frontend does not surface edit
    /// controls to non-owners, so the authorizer treats any non-owner
    /// modification as denied. Moderators retain read access (the
    /// room view itself); plain invitees and invite codes are denied —
    /// they reach the room through the meeting-time `/rooms/{room_id}/start`
    /// endpoint, not through this metadata resource.
    ///
    /// ```text
    /// | Subject               | Access |
    /// |-----------------------|--------|
    /// | Owner                 | rw     |
    /// | Moderator             | r-     |
    /// | User                  | --     |
    /// | Unrelated User        | --     |
    /// | Valid Invite Code     | --     |
    /// | Invalid Invite Code   | --     |
    /// ```
    ///
    /// [`Room`]: opentalk_controller_api_authorization::authorization::Resource::Room
    pub(crate) async fn authorize_room(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::Read,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        self.apply_acl_for_room(subjects, method, room_id, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mockall::predicate::eq;
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Get, Patch},
        Admission::{self, Allowed, Denied},
        AuthorizationTarget, AuthorizerBackend, Resource, Subject, SubjectCollection,
    };
    use opentalk_controller_settings::settings_provider_from_example_raw_settings;
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
        INVITE_CODE, ROOM_ID, USER_ID, create_authorizer_with_role, create_authorizer_with_validity,
    };
    use crate::{
        OpenTalkAuthorizerBackend,
        event::test_utils::{DISABLED_FEATURES, MODULE_FEATURES},
    };

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, Get, Allowed)]
    #[case::owner_patch(Owner, Patch, Allowed)]
    #[case::moderator_get(Invited(Moderator), Get, Allowed)]
    #[case::moderator_patch(Invited(Moderator), Patch, Denied)]
    #[case::user_get(Invited(User), Get, Denied)]
    #[case::user_patch(Invited(User), Patch, Denied)]
    #[case::unrelated_get(Unrelated, Get, Denied)]
    #[case::unrelated_patch(Unrelated, Patch, Denied)]
    async fn user(
        #[case] role: AuthorizationUserRole,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_role(role);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(USER_ID)]),
                resource: Resource::Room(ROOM_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, Get, Denied)]
    #[case::valid_patch(Valid, Patch, Denied)]
    #[case::invalid_get(Invalid, Get, Denied)]
    #[case::invalid_patch(Invalid, Patch, Denied)]
    async fn invite_code(
        #[case] validity: AuthorizationInviteCodeValidity,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_validity(validity);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(INVITE_CODE)]),
                resource: Resource::Room(ROOM_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    async fn multiple_subjects_first_allowed_wins() {
        // Moderator (a valid user role on the room) reads succeed; we
        // include both an invite code and the user subject to verify
        // the user branch wins without short-circuiting incorrectly.
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_room_user_role()
            .with(eq(ROOM_ID), eq(USER_ID))
            .return_once(move |_, _| Ok(Invited(Moderator)));
        let inventory: Box<dyn AuthorizationInventory> = Box::new(inventory);

        let mut inventory_provider = MockInventoryProvider::new();
        let _ = inventory_provider
            .expect_get_authorization_inventory()
            .return_once(move || Ok(inventory));

        let settings_provider = settings_provider_from_example_raw_settings();
        let authorizer = OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            settings_provider,
            MODULE_FEATURES,
        );

        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([
                    Subject::from(USER_ID),
                    Subject::from(INVITE_CODE),
                ]),
                resource: Resource::Room(ROOM_ID),
                access_method: Get,
            })
            .await
            .unwrap();
        assert_eq!(Allowed, admission);
    }

    #[tokio::test]
    async fn multiple_subjects_all_denied() {
        let mut inventory = MockAuthorizationInventory::new();
        let _ = inventory
            .expect_get_room_user_role()
            .with(eq(ROOM_ID), eq(USER_ID))
            .return_once(move |_, _| Ok(Unrelated));
        let _ = inventory
            .expect_get_room_invite_code_validity()
            .with(
                eq(ROOM_ID),
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

        let settings_provider = settings_provider_from_example_raw_settings();
        let authorizer = OpenTalkAuthorizerBackend::new(
            Arc::new(inventory_provider),
            settings_provider,
            MODULE_FEATURES,
        );

        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([
                    Subject::from(USER_ID),
                    Subject::from(INVITE_CODE),
                ]),
                resource: Resource::Room(ROOM_ID),
                access_method: Get,
            })
            .await
            .unwrap();
        assert_eq!(Denied, admission);
    }
}
