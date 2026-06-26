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
    /// Authorize access to an [`EventUserInvite`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`EventUserInvite`] resource (an individual invite addressed to a
    /// specific OpenTalk user). Only the owner may read and write.
    ///
    /// ```text
    /// | Subject                 | Access |
    /// | ----------------------- | ------ |
    /// | **Owner**               | rw     |
    /// | **Moderator**           | --     |
    /// | **Invited-User**        | --     |
    /// | **Unrelated-User**      | --     |
    /// | **Valid Invite-Code**   | --     |
    /// | **Invalid Invite-Code** | --     |
    /// ```
    ///
    /// [`EventUserInvite`]: opentalk_controller_api_authorization::authorization::Resource::EventUserInvite
    pub(crate) async fn authorize_event_user_invite(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::None,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        self.apply_acl_for_event(subjects, method, event_id, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Delete, Get, Patch},
        Admission::{self, Allowed, Denied},
        AuthorizationTarget, AuthorizerBackend, Resource, Subject, SubjectCollection,
    };
    use opentalk_inventory::{
        AuthorizationInviteCodeValidity::{self, Invalid, Valid},
        AuthorizationUserRole::{self, Invited, Owner, Unrelated},
    };
    use opentalk_types_common::{
        events::invites::InviteRole::{Moderator, User},
        users::UserId,
    };
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::super::test_utils::{
        EVENT_ID, INVITE_CODE, USER_ID, create_authorizer_with_role,
        create_authorizer_with_validity,
    };

    /// A different user id for the path segment, demonstrating that
    /// the invitee's identity does not affect authorization.
    const OTHER_USER_ID: UserId = UserId::from_u128(0x0099);

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, Get, Allowed)]
    #[case::owner_patch(Owner, Patch, Allowed)]
    #[case::owner_delete(Owner, Delete, Allowed)]
    #[case::moderator_get(Invited(Moderator), Get, Denied)]
    #[case::moderator_patch(Invited(Moderator), Patch, Denied)]
    #[case::moderator_delete(Invited(Moderator), Delete, Denied)]
    #[case::user_get(Invited(User), Get, Denied)]
    #[case::user_patch(Invited(User), Patch, Denied)]
    #[case::user_delete(Invited(User), Delete, Denied)]
    #[case::unrelated_get(Unrelated, Get, Denied)]
    #[case::unrelated_patch(Unrelated, Patch, Denied)]
    #[case::unrelated_delete(Unrelated, Delete, Denied)]
    async fn user(
        #[case] role: AuthorizationUserRole,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_role(role);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(USER_ID)]),
                resource: Resource::EventUserInvite(EVENT_ID, OTHER_USER_ID),
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
    #[case::valid_delete(Valid, Delete, Denied)]
    #[case::invalid_get(Invalid, Get, Denied)]
    #[case::invalid_patch(Invalid, Patch, Denied)]
    #[case::invalid_delete(Invalid, Delete, Denied)]
    async fn invite_code(
        #[case] validity: AuthorizationInviteCodeValidity,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_validity(validity);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(INVITE_CODE)]),
                resource: Resource::EventUserInvite(EVENT_ID, OTHER_USER_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
