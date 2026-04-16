// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{rooms::invite_codes::InviteCode, users::UserId};

/// A subject for which the access to resources can be checked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Subject {
    /// An authenticated user.
    User(UserId),

    /// An invite code.
    InviteCode(InviteCode),
}

impl Subject {
    /// Check whether the subject is a user
    pub fn is_user(&self) -> bool {
        matches!(self, Subject::User(_))
    }

    /// Check whether the subject is an invite code
    pub fn is_invite_code(&self) -> bool {
        matches!(self, Subject::InviteCode(_))
    }
}

impl From<UserId> for Subject {
    fn from(user_id: UserId) -> Self {
        Self::User(user_id)
    }
}

impl From<&UserId> for Subject {
    fn from(user_id: &UserId) -> Self {
        Self::from(*user_id)
    }
}

impl From<InviteCode> for Subject {
    fn from(invite_code: InviteCode) -> Self {
        Self::InviteCode(invite_code)
    }
}

impl From<&InviteCode> for Subject {
    fn from(invite_code: &InviteCode) -> Self {
        Self::from(*invite_code)
    }
}
