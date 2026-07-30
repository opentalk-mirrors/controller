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
    /// Authorize access to a [`RoomTariff`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`RoomTariff`] resource (`GET` on
    /// `/rooms/{room_id}/tariff`). The tariff exposes meeting
    /// capabilities (max participants, available features) that
    /// every joining party needs in order to decide what UI to
    /// render. Anonymous guests entering through a valid invite
    /// code are therefore granted read access too — denying them
    /// here forces the frontend into a guess-and-check workflow.
    /// The endpoint has no write surface.
    ///
    /// ```text
    /// | Subject               | Access |
    /// |-----------------------|--------|
    /// | Owner                 | r-     |
    /// | Moderator             | r-     |
    /// | User                  | r-     |
    /// | Unrelated User        | --     |
    /// | Valid Invite Code     | r-     |
    /// | Invalid Invite Code   | --     |
    /// ```
    ///
    /// [`RoomTariff`]: opentalk_controller_api_authorization::authorization::Resource::RoomTariff
    pub(crate) async fn authorize_room_tariff(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        room_id_or_alias: RoomIdOrAlias,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::Read,
            moderator: Access::Read,
            invited_user: Access::Read,
            invite_code: Access::Read,
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
    #[case::owner_post(Owner, Post, Denied)]
    #[case::moderator_get(Invited(Moderator), Get, Allowed)]
    #[case::moderator_post(Invited(Moderator), Post, Denied)]
    #[case::user_get(Invited(User), Get, Allowed)]
    #[case::user_post(Invited(User), Post, Denied)]
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
                resource: Resource::RoomTariff(ROOM_ID.into()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, Get, Allowed)]
    #[case::valid_post(Valid, Post, Denied)]
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
                resource: Resource::RoomTariff(ROOM_ID.into()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
