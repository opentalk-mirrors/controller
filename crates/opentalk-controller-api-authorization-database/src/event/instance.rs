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
    /// Authorize access to an [`EventInstance`] resource.
    ///
    /// # Access Control List
    ///
    /// Access rights for the [`EventInstance`] resource (a single instance of a recurring event,
    /// identified by `instance_id`). Same access model as [`Event`] — the `instance_id` segment has
    /// no impact on authorization, only on which instance the request mutates.
    ///
    /// API methods exposed are `GET` and `PATCH`. Only the owner may `PATCH`; any associated
    /// subject (Owner, Invited(_), Valid invite code) may `GET`.
    ///
    /// ```text
    /// | Subject                 | Access |
    /// | ----------------------- | ------ |
    /// | **Owner**               | rw     |
    /// | **Moderator**           | r-     |
    /// | **Invited-User**        | r-     |
    /// | **Unrelated-User**      | --     |
    /// | **Valid Invite-Code**   | r-     |
    /// | **Invalid Invite-Code** | --     |
    /// ```
    ///
    /// [`EventInstance`]: opentalk_controller_api_authorization::authorization::Resource::EventInstance
    /// [`Event`]: opentalk_controller_api_authorization::authorization::Resource::Event
    pub(crate) async fn authorize_event_instance(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
    ) -> Result<Admission> {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::Read,
            invited_user: Access::Read,
            invite_code: Access::Read,
        };

        self.apply_acl_for_event(subjects, method, event_id, acl)
            .await
    }
}

#[cfg(test)]
mod tests {
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Get, Patch},
        Admission::{self, Allowed, Denied},
        AuthorizationTarget, AuthorizerBackend, Resource, Subject, SubjectCollection,
    };
    use opentalk_inventory::{
        AuthorizationInviteCodeValidity::{self, Invalid, Valid},
        AuthorizationUserRole::{self, Invited, Owner, Unrelated},
    };
    use opentalk_types_api_v1::events::InstanceId;
    use opentalk_types_common::{
        events::invites::InviteRole::{Moderator, User},
        time::Timestamp,
    };
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::super::test_utils::{
        EVENT_ID, INVITE_CODE, USER_ID, create_authorizer_with_role,
        create_authorizer_with_validity,
    };

    /// A constant `InstanceId` used by tests. Its value is irrelevant
    /// to authorization — every event instance inherits the event's
    /// ACL — but `InstanceId`'s fields are private, so we construct
    /// it from a stable timestamp.
    fn instance_id() -> InstanceId {
        InstanceId::from(Timestamp::unix_epoch())
    }

    #[tokio::test]
    #[rstest]
    #[case::owner_get(Owner, Get, Allowed)]
    #[case::owner_patch(Owner, Patch, Allowed)]
    #[case::moderator_get(Invited(Moderator), Get, Allowed)]
    #[case::moderator_patch(Invited(Moderator), Patch, Denied)]
    #[case::user_get(Invited(User), Get, Allowed)]
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
                resource: Resource::EventInstance(EVENT_ID, instance_id()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }

    #[tokio::test]
    #[rstest]
    #[case::valid_get(Valid, Get, Allowed)]
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
                resource: Resource::EventInstance(EVENT_ID, instance_id()),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(expected_admission, admission);
    }
}
