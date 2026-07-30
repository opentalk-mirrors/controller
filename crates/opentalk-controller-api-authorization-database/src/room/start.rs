// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, SubjectCollection,
};
use opentalk_types_common::rooms::RoomIdOrAlias;

use crate::{
    OpenTalkAuthorizerBackend, Result,
    common::acl::{Access, Acl},
};

impl OpenTalkAuthorizerBackend {
    /// Authorize access to a [`RoomStart`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`RoomStart`] resource (`POST` on
    /// `/rooms/{room_id}/start`). This is the meeting join endpoint:
    /// every participant — including those entering through an
    /// invite code — must be able to call `POST` to take part. The
    /// matrix is therefore `ReadWrite` for every subject that has any
    /// relationship to the room. Treating the call as "write" is a
    /// loose fit for an HTTP method, but the ACL semantics line up
    /// with the operational behaviour: if a subject is allowed to
    /// join, they are allowed to invoke this endpoint.
    ///
    /// ```text
    /// | Subject               | Access |
    /// |-----------------------|--------|
    /// | Owner                 | rw     |
    /// | Moderator             | rw     |
    /// | User                  | rw     |
    /// | Unrelated User        | --     |
    /// | Valid Invite Code     | rw     |
    /// | Invalid Invite Code   | --     |
    /// ```
    ///
    /// [`RoomStart`]: opentalk_controller_api_authorization::authorization::Resource::RoomStart
    pub(crate) async fn authorize_room_start(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        room_id_or_alias: RoomIdOrAlias,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::ReadWrite,
            invited_user: Access::ReadWrite,
            invite_code: Access::ReadWrite,
        };

        self.apply_acl_for_room(subjects, method, room_id_or_alias, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Get, Post},
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
        INVITE_CODE, ROOM_ID, USER_ID, create_authorizer_with_role, create_authorizer_with_validity,
    };

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, Get, Allowed)]
    #[case::owner_post(Owner, Post, Allowed)]
    #[case::moderator_get(Invited(Moderator), Get, Allowed)]
    #[case::moderator_post(Invited(Moderator), Post, Allowed)]
    #[case::user_get(Invited(User), Get, Allowed)]
    #[case::user_post(Invited(User), Post, Allowed)]
    #[case::unrelated_get(Unrelated, Get, Denied)]
    #[case::unrelated_post(Unrelated, Post, Denied)]
    async fn user(
        #[case] role: AuthorizationUserRole,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_role(role);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(USER_ID)]),
                resource: Resource::RoomStart(ROOM_ID.into()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, Get, Allowed)]
    #[case::valid_post(Valid, Post, Allowed)]
    #[case::invalid_get(Invalid, Get, Denied)]
    #[case::invalid_post(Invalid, Post, Denied)]
    async fn invite_code(
        #[case] validity: AuthorizationInviteCodeValidity,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_validity(validity);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(INVITE_CODE)]),
                resource: Resource::RoomStart(ROOM_ID.into()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
