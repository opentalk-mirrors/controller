// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, SubjectCollection,
};
use opentalk_types_common::rooms::{RoomId, invite_codes::InviteCode};

use crate::{
    OpenTalkAuthorizerBackend, Result,
    common::acl::{Access, Acl},
};

impl OpenTalkAuthorizerBackend {
    /// Authorize access to a [`RoomInviteCode`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`RoomInviteCode`] resource (`GET`/`PUT`/`DELETE` on
    /// `/rooms/{room_id}/invites/{code}`). The endpoint exposes invite-code metadata (expiry, role,
    /// generated URL) but not the surrounding invite list.
    ///
    /// ```text
    /// | Subject               | Access |
    /// |-----------------------|--------|
    /// | Owner                 | rw     |
    /// | Moderator             | --     |
    /// | User                  | --     |
    /// | Unrelated User        | --     |
    /// | Valid Invite Code     | --     |
    /// | Invalid Invite Code   | --     |
    /// ```
    ///
    /// [`RoomInviteCode`]: opentalk_controller_api_authorization::authorization::Resource::RoomInviteCode
    pub(crate) async fn authorize_room_invite_code(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
        _invite_code: InviteCode,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::None,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        self.apply_acl_for_room(subjects, method, room_id, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Delete, Get, Put},
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
    #[case::owner_put(Owner, Put, Allowed)]
    #[case::owner_delete(Owner, Delete, Allowed)]
    #[case::moderator_get(Invited(Moderator), Get, Denied)]
    #[case::moderator_put(Invited(Moderator), Put, Denied)]
    #[case::moderator_delete(Invited(Moderator), Delete, Denied)]
    #[case::user_get(Invited(User), Get, Denied)]
    #[case::user_put(Invited(User), Put, Denied)]
    #[case::user_delete(Invited(User), Delete, Denied)]
    #[case::unrelated_get(Unrelated, Get, Denied)]
    #[case::unrelated_put(Unrelated, Put, Denied)]
    async fn user(
        #[case] role: AuthorizationUserRole,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_role(role);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(USER_ID)]),
                resource: Resource::RoomInviteCode(ROOM_ID, INVITE_CODE),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, Get, Denied)]
    #[case::valid_put(Valid, Put, Denied)]
    #[case::valid_delete(Valid, Delete, Denied)]
    #[case::invalid_get(Invalid, Get, Denied)]
    #[case::invalid_put(Invalid, Put, Denied)]
    async fn invite_code(
        #[case] validity: AuthorizationInviteCodeValidity,
        #[case] access_method: AccessMethod,
        #[case] expected_admission: Admission,
    ) {
        let authorizer = create_authorizer_with_validity(validity);
        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: SubjectCollection::from_iter([Subject::from(INVITE_CODE)]),
                resource: Resource::RoomInviteCode(ROOM_ID, INVITE_CODE),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
