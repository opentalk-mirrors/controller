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
    /// Authorize access to an [`EventInvite`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`EventInvite`] resource (the singular per-event invite endpoint that
    /// an invitee uses to accept or decline). The owner is denied: an event's creator can not
    /// accept or decline their own event. Any invited user (Moderator or User) is allowed. Invite
    /// codes are denied.
    ///
    /// ```text
    /// | Subject                 | Access |
    /// | ----------------------- | ------ |
    /// | **Owner**               | --     |
    /// | **Moderator**           | rw     |
    /// | **Invited-User**        | rw     |
    /// | **Unrelated-User**      | --     |
    /// | **Valid Invite-Code**   | --     |
    /// | **Invalid Invite-Code** | --     |
    /// ```
    ///
    /// [`EventInvite`]: opentalk_controller_api_authorization::authorization::Resource::EventInvite
    pub(crate) async fn authorize_event_invite(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::None,
            moderator: Access::ReadWrite,
            invited_user: Access::ReadWrite,
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
    use opentalk_types_common::events::invites::InviteRole::{Moderator, User};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::super::test_utils::{
        EVENT_ID, INVITE_CODE, USER_ID, create_authorizer_with_role,
        create_authorizer_with_validity,
    };

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, Get, Denied)]
    #[case::owner_patch(Owner, Patch, Denied)]
    #[case::owner_delete(Owner, Delete, Denied)]
    #[case::moderator_get(Invited(Moderator), Get, Allowed)]
    #[case::moderator_patch(Invited(Moderator), Patch, Allowed)]
    #[case::moderator_delete(Invited(Moderator), Delete, Allowed)]
    #[case::user_get(Invited(User), Get, Allowed)]
    #[case::user_patch(Invited(User), Patch, Allowed)]
    #[case::user_delete(Invited(User), Delete, Allowed)]
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
                resource: Resource::EventInvite(EVENT_ID),
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
                resource: Resource::EventInvite(EVENT_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
