// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeMap;

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, Subject, SubjectCollection,
};
use opentalk_types_common::{events::invites::InviteRole, users::UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub owner: UserId,
    pub invited_users: BTreeMap<UserId, InviteRole>,
}

impl Event {
    pub fn new(owner: UserId) -> Self {
        Self {
            owner,
            invited_users: BTreeMap::new(),
        }
    }

    pub fn add_invited_user(&mut self, user: &UserId, role: &InviteRole) {
        let _ = self.invited_users.insert(*user, *role);
    }

    pub fn remove_invited_user(&mut self, user: &UserId) {
        let _ = self.invited_users.remove(user);
    }

    pub fn authorize(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        // Attempt authorization against owner
        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Allowed;
        }

        // Attempt authorization against invited users
        if authenticated_subjects
            .any_user_has_role_or_higher(&self.invited_users, method.required_invite_role())
        {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_invite(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        if ![AccessMethod::Patch, AccessMethod::Delete].contains(&method) {
            return Admission::Denied;
        }

        // The owner of a room can't accept or decline an invite
        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Denied;
        }

        if authenticated_subjects.contains_any_user_by_key(&self.invited_users) {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_shared_folder(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        if ![AccessMethod::Get, AccessMethod::Put, AccessMethod::Delete].contains(&method) {
            return Admission::Denied;
        }

        // The owner of an event is allowed to modify the shared folder
        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Allowed;
        }

        // Invited users are only allowed to read
        if method.requires_write_access() {
            return Admission::Denied;
        }

        if authenticated_subjects.contains_any_user_by_key(&self.invited_users) {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_favorite(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        if ![AccessMethod::Put, AccessMethod::Delete].contains(&method) {
            return Admission::Denied;
        }

        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Allowed;
        }

        if authenticated_subjects.contains_any_user_by_key(&self.invited_users) {
            return Admission::Allowed;
        }

        Admission::Denied
    }
}
