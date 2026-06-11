// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Cross-resource ACL helpers shared by `event/`, `room/` and `user/`.

use acl::Acl;
use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, Subject, SubjectCollection,
};
use opentalk_inventory::{
    AuthorizationInviteCodeValidity as Validity, AuthorizationUserRole as Role,
};
use opentalk_types_common::{
    events::{EventId, invites::InviteRole},
    rooms::RoomId,
};

use crate::{OpenTalkAuthorizerBackend, Result};

impl OpenTalkAuthorizerBackend {
    /// Helper for ACL pattern: any registered user can access; invite codes can not.
    ///
    /// This helper is used by resources that don't fit the Owner/Moderator/Member/Invited model,
    /// but should be accessible to any registered user, regardless of access method. Examples
    /// include the `Rooms` and `Events` listing endpoints, or the `UserMe` and `UserProfile`
    /// resources that expose write paths.
    pub(crate) fn require_read_write_user(subjects: SubjectCollection) -> Admission {
        for subject in subjects.0 {
            if let Subject::User(_) = subject {
                return Admission::Allowed;
            }
        }
        Admission::Denied
    }

    /// Helper for ACL pattern: any registered user can read; invite codes can not; nobody writes.
    ///
    /// Read-only variant of [`Self::require_write_user`] for endpoints that don't fit the
    /// Owner/Moderator/Invited/InviteCode model and expose no write methods. Examples include
    /// `UserFind` (`GET /v1/users/find`) and the `UserMeTariff` / `UserMePendingInvites` /
    /// `UserMeAssets` endpoints, which all only expose `GET`.
    pub(crate) fn require_read_user(
        subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        for subject in subjects.0 {
            if let Subject::User(_) = subject
                && method.is_read_only()
            {
                return Admission::Allowed;
            }
        }
        Admission::Denied
    }

    pub(crate) async fn apply_acl_for_event(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        event_id: EventId,
        acl: Acl,
    ) -> Result<Admission> {
        let mut inventory = self.inventory.get_authorization_inventory().await?;

        for subject in subjects.0 {
            let admission = match subject {
                Subject::User(user_id) => {
                    let role = inventory.get_event_user_role(event_id, user_id).await?;
                    acl.apply(role.into(), method)
                }
                Subject::InviteCode(invite_code) => {
                    let module_features = self.module_features.clone();
                    let disabled_features = self.settings.get().defaults.disabled_features.clone();
                    let validity = inventory
                        .get_event_invite_code_validity(
                            event_id,
                            invite_code,
                            disabled_features,
                            module_features,
                        )
                        .await?;
                    acl.apply(validity.into(), method)
                }
            };

            if admission.is_allowed() {
                return Ok(admission);
            }
        }

        Ok(Admission::Denied)
    }

    pub(crate) async fn apply_acl_for_room(
        &self,
        subjects: SubjectCollection,
        method: AccessMethod,
        room_id: RoomId,
        acl: Acl,
    ) -> Result<Admission> {
        let mut inventory = self.inventory.get_authorization_inventory().await?;

        for subject in subjects.0 {
            let admission = match subject {
                Subject::User(user_id) => {
                    let role = inventory.get_room_user_role(room_id, user_id).await?;
                    acl.apply(role.into(), method)
                }
                Subject::InviteCode(invite_code) => {
                    let module_features = self.module_features.clone();
                    let disabled_features = self.settings.get().defaults.disabled_features.clone();
                    let validity = inventory
                        .get_room_invite_code_validity(
                            room_id,
                            invite_code,
                            disabled_features,
                            module_features,
                        )
                        .await?;
                    acl.apply(validity.into(), method)
                }
            };

            if admission.is_allowed() {
                return Ok(admission);
            }
        }

        Ok(Admission::Denied)
    }
}

pub(crate) mod acl {
    use super::*;

    #[derive(Debug, Clone)]
    pub enum Access {
        ReadWrite,
        Read,
        None,
    }

    /// # Access Control List
    ///
    /// Used to enforce [`Access`] to specific resources, i.e., API endpoints that relate to created
    /// events or rooms. Note that a [`Subject`] that does not map to [`Acl::owner`],
    /// [`Acl::moderator`], [`Acl::invited_user`], or [`Acl::invite_code`] is denied
    /// admission ([`Admission::Denied`]) by default.
    #[derive(Debug, Clone)]
    pub struct Acl {
        pub owner: Access,
        pub moderator: Access,
        pub invited_user: Access,
        pub invite_code: Access,
    }

    impl Acl {
        pub(super) fn apply(&self, subject: Subject, method: AccessMethod) -> Admission {
            let access = self.get_access(subject);

            match (access, method) {
                (Access::ReadWrite, _) => Admission::Allowed,
                (Access::Read, method) if method.is_read_only() => Admission::Allowed,
                (Access::Read, _) | (Access::None, _) => Admission::Denied,
            }
        }

        fn get_access(&self, subject: Subject) -> &Access {
            match subject {
                Subject::User(role) => match role {
                    Role::Owner => &self.owner,
                    Role::Invited(InviteRole::Moderator) => &self.moderator,
                    Role::Invited(InviteRole::User) => &self.invited_user,
                    Role::Unrelated => &Access::None,
                },
                Subject::InviteCode(validity) => match validity {
                    Validity::Valid => &self.invite_code,
                    Validity::Invalid => &Access::None,
                },
            }
        }
    }

    #[derive(Debug, Clone)]
    pub(super) enum Subject {
        User(Role),
        InviteCode(Validity),
    }

    impl From<Role> for Subject {
        fn from(value: Role) -> Self {
            Self::User(value)
        }
    }

    impl From<Validity> for Subject {
        fn from(value: Validity) -> Self {
            Self::InviteCode(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use AccessMethod::{Get, Post};
    use Admission::{Allowed, Denied};
    use acl::Subject::{InviteCode, User};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::{
        common::acl::Access,
        event::test_utils::{INVITE_CODE, USER_ID},
    };

    #[rstest]
    #[case::apply_owner(User(Role::Owner), Allowed, Allowed)]
    #[case::apply_moderator(User(Role::Invited(InviteRole::Moderator)), Denied, Denied)]
    #[case::apply_invited_user(User(Role::Invited(InviteRole::User)), Denied, Denied)]
    #[case::apply_unrelated_user(User(Role::Unrelated), Denied, Denied)]
    #[case::apply_invite_code(InviteCode(Validity::Valid), Denied, Denied)]
    fn apply_acl_read_write_owner(
        #[case] sub: acl::Subject,
        #[case] expected_get: Admission,
        #[case] expected_post: Admission,
    ) {
        let acl = Acl {
            owner: Access::ReadWrite,
            moderator: Access::None,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        assert_eq!(expected_get, acl.apply(sub.clone(), Get));
        assert_eq!(expected_post, acl.apply(sub, Post));
    }

    #[rstest]
    #[case::apply_owner(User(Role::Owner), Denied, Denied)]
    #[case::apply_moderator(User(Role::Invited(InviteRole::Moderator)), Allowed, Allowed)]
    #[case::apply_invited_user(User(Role::Invited(InviteRole::User)), Denied, Denied)]
    #[case::apply_unrelated_user(User(Role::Unrelated), Denied, Denied)]
    #[case::apply_invite_code(InviteCode(Validity::Valid), Denied, Denied)]
    fn apply_acl_read_write_moderator(
        #[case] sub: acl::Subject,
        #[case] expected_get: Admission,
        #[case] expected_post: Admission,
    ) {
        let acl = Acl {
            owner: Access::None,
            moderator: Access::ReadWrite,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        assert_eq!(expected_get, acl.apply(sub.clone(), Get));
        assert_eq!(expected_post, acl.apply(sub, Post));
    }

    #[rstest]
    #[case::apply_owner(User(Role::Owner), Denied, Denied)]
    #[case::apply_moderator(User(Role::Invited(InviteRole::Moderator)), Denied, Denied)]
    #[case::apply_invited_user(User(Role::Invited(InviteRole::User)), Allowed, Allowed)]
    #[case::apply_unrelated_user(User(Role::Unrelated), Denied, Denied)]
    #[case::apply_invite_code(InviteCode(Validity::Valid), Denied, Denied)]
    fn apply_acl_read_write_invited_user(
        #[case] sub: acl::Subject,
        #[case] expected_get: Admission,
        #[case] expected_post: Admission,
    ) {
        let acl = Acl {
            owner: Access::None,
            moderator: Access::None,
            invited_user: Access::ReadWrite,
            invite_code: Access::None,
        };

        assert_eq!(expected_get, acl.apply(sub.clone(), Get));
        assert_eq!(expected_post, acl.apply(sub, Post));
    }

    #[rstest]
    #[case::apply_owner(User(Role::Owner), Denied, Denied)]
    #[case::apply_moderator(User(Role::Invited(InviteRole::Moderator)), Denied, Denied)]
    #[case::apply_invited_user(User(Role::Invited(InviteRole::User)), Denied, Denied)]
    #[case::apply_unrelated_user(User(Role::Unrelated), Denied, Denied)]
    #[case::apply_invite_code(InviteCode(Validity::Valid), Allowed, Allowed)]
    fn apply_acl_read_write_invite_code(
        #[case] sub: acl::Subject,
        #[case] expected_get: Admission,
        #[case] expected_post: Admission,
    ) {
        let acl = Acl {
            owner: Access::None,
            moderator: Access::None,
            invited_user: Access::None,
            invite_code: Access::ReadWrite,
        };

        assert_eq!(expected_get, acl.apply(sub.clone(), Get));
        assert_eq!(expected_post, acl.apply(sub, Post));
    }

    #[rstest]
    #[case::apply_owner(User(Role::Owner), Denied, Denied)]
    #[case::apply_moderator(User(Role::Invited(InviteRole::Moderator)), Denied, Denied)]
    #[case::apply_invited_user(User(Role::Invited(InviteRole::User)), Denied, Denied)]
    #[case::apply_unrelated_user(User(Role::Unrelated), Denied, Denied)]
    #[case::apply_invite_code(InviteCode(Validity::Valid), Denied, Denied)]
    fn apply_acl_read_write_none(
        #[case] sub: acl::Subject,
        #[case] expected_get: Admission,
        #[case] expected_post: Admission,
    ) {
        let acl = Acl {
            owner: Access::None,
            moderator: Access::None,
            invited_user: Access::None,
            invite_code: Access::None,
        };

        assert_eq!(expected_get, acl.apply(sub.clone(), Get));
        assert_eq!(expected_post, acl.apply(sub, Post));
    }

    #[rstest]
    #[case::user_get(Subject::User(USER_ID), Get, Allowed)]
    #[case::user_post(Subject::User(USER_ID), Post, Denied)]
    #[case::invite_code_get(Subject::InviteCode(INVITE_CODE), Get, Denied)]
    #[case::invite_code_get(Subject::InviteCode(INVITE_CODE), Post, Denied)]
    fn require_read_user(
        #[case] sub: Subject,
        #[case] method: AccessMethod,
        #[case] admission: Admission,
    ) {
        let subjects = SubjectCollection::from_iter([sub]);
        assert_eq!(
            OpenTalkAuthorizerBackend::require_read_user(subjects, method),
            admission
        );
    }

    #[rstest]
    #[case::user_get(Subject::User(USER_ID), Allowed)]
    #[case::invite_code_get(Subject::InviteCode(INVITE_CODE), Denied)]
    fn require_write_user(#[case] sub: Subject, #[case] admission: Admission) {
        let subjects = SubjectCollection::from_iter([sub]);
        assert_eq!(
            OpenTalkAuthorizerBackend::require_read_write_user(subjects),
            admission
        );
    }
}
