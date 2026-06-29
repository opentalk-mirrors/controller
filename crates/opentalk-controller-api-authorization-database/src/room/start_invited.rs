// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_controller_api_authorization::authorization::Admission;

use crate::OpenTalkAuthorizerBackend;

impl OpenTalkAuthorizerBackend {
    /// Authorize access to the [`RoomStartInvited`] resource (`POST` on
    /// `/rooms/{room_id}/start_invited`).
    ///
    /// Unconditionally allowed: the endpoint just issues a permanent
    /// redirect to [`RoomStart`] — the real authorization check happens
    /// there, fed by the credentials the client presents on the
    /// follow-up request. Permitting the redirect itself does not grant
    /// access to the room.
    ///
    /// ```text
    /// | Subject                 | Access |
    /// | ----------------------- | ------ |
    /// | **Unauthenticated**     | rw     |
    /// | **User**                | rw     |
    /// | **Invite-Code**         | rw     |
    /// ```
    ///
    /// [`RoomStartInvited`]: opentalk_controller_api_authorization::authorization::Resource::RoomStartInvited
    /// [`RoomStart`]: opentalk_controller_api_authorization::authorization::Resource::RoomStart
    pub(crate) const fn authorize_room_start_invited() -> Admission {
        Admission::Allowed
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use opentalk_controller_api_authorization::authorization::{
        AccessMethod::{self, Get, Post},
        Admission::Allowed,
        AuthorizationTarget, AuthorizerBackend, Resource, Subject, SubjectCollection,
    };
    use opentalk_controller_settings::test_util;
    use opentalk_inventory::MockInventoryProvider;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use crate::{
        OpenTalkAuthorizerBackend,
        event::test_utils::MODULE_FEATURES,
        room::test_utils::{INVITE_CODE, ROOM_ID, USER_ID},
    };

    #[tokio::test]
    #[rstest]
    #[case::unauth_get(SubjectCollection::default(), Get)]
    #[case::unauth_post(SubjectCollection::default(), Post)]
    #[case::user_get(SubjectCollection::from_iter([Subject::from(USER_ID)]), Get)]
    #[case::user_post(SubjectCollection::from_iter([Subject::from(USER_ID)]), Post)]
    #[case::invite_get(SubjectCollection::from_iter([Subject::from(INVITE_CODE)]), Get)]
    #[case::invite_post(SubjectCollection::from_iter([Subject::from(INVITE_CODE)]), Post)]
    async fn room_start_invited_is_unconditionally_allowed(
        #[case] subjects: SubjectCollection,
        #[case] access_method: AccessMethod,
    ) {
        // The authorizer must not consult the inventory — using a bare
        // `MockInventoryProvider` without expectations makes that
        // contract explicit: any call into the inventory would panic.
        let authorizer = OpenTalkAuthorizerBackend::new(
            Arc::new(MockInventoryProvider::new()),
            test_util::settings_provider_from_example_raw_settings(),
            MODULE_FEATURES,
        );

        let admission = authorizer
            .authorize(AuthorizationTarget {
                authenticated_subjects: subjects,
                resource: Resource::RoomStartInvited(ROOM_ID),
                access_method,
            })
            .await
            .unwrap();
        assert_eq!(
            Allowed, admission,
            "RoomStartInvited must always be allowed"
        );
    }
}
