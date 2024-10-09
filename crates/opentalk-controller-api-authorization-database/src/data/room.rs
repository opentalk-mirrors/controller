// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::{BTreeMap, BTreeSet};

use opentalk_controller_api_authorization::authorization::{
    AccessMethod, Admission, Subject, SubjectCollection,
};
use opentalk_types_common::{
    events::invites::InviteRole, rooms::invite_codes::InviteCode, users::UserId,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    pub owner: UserId,
    pub invited_users: BTreeMap<UserId, InviteRole>,
    pub invite_codes: BTreeSet<InviteCode>,
}

impl Room {
    pub fn new(owner: UserId) -> Self {
        Self {
            owner,
            invited_users: BTreeMap::new(),
            invite_codes: BTreeSet::new(),
        }
    }

    pub fn add_invited_user(&mut self, user: &UserId, role: &InviteRole) {
        let _ = self.invited_users.insert(*user, *role);
    }

    pub fn remove_invited_user(&mut self, user: &UserId) {
        let _ = self.invited_users.remove(user);
    }

    pub fn add_invite_code(&mut self, invite_code: &InviteCode) {
        let _ = self.invite_codes.insert(*invite_code);
    }

    pub fn remove_invite_code(&mut self, invite_code: &InviteCode) {
        let _ = self.invite_codes.remove(invite_code);
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

        // Invite codes are only allowed to read
        if method.is_read_only()
            && authenticated_subjects.contains_any_invite_code(&self.invite_codes)
        {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_invite_code(
        &self,
        authenticated_subjects: SubjectCollection,
        _method: AccessMethod,
    ) -> Admission {
        // Attempt authorization against owner
        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_start(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        if method != AccessMethod::Post {
            return Admission::Denied;
        }

        // Attempt authorization against owner
        if authenticated_subjects.contains(&Subject::User(self.owner)) {
            return Admission::Allowed;
        }

        if authenticated_subjects.contains_any_user_by_key(&self.invited_users) {
            return Admission::Allowed;
        }

        Admission::Denied
    }

    pub fn authorize_tariff(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        if method.requires_write_access() {
            return Admission::Denied;
        }
        self.authorize(authenticated_subjects, method)
    }

    pub fn authorize_assets(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        self.authorize(authenticated_subjects, method)
    }

    pub fn authorize_asset(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        self.authorize(authenticated_subjects, method)
    }

    pub fn authorize_streaming_targets(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        self.authorize(authenticated_subjects, method)
    }

    pub fn authorize_streaming_target(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        self.authorize(authenticated_subjects, method)
    }

    pub fn authorize_sip(
        &self,
        authenticated_subjects: SubjectCollection,
        method: AccessMethod,
    ) -> Admission {
        self.authorize(authenticated_subjects, method)
    }
}
