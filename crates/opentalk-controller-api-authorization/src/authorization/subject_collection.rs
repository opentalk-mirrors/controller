// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::{BTreeMap, BTreeSet};

use opentalk_types_common::{
    events::invites::InviteRole, rooms::invite_codes::InviteCode, time::Timestamp, users::UserId,
};

use crate::authorization::Subject;

/// A collection of subjects.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubjectCollection(pub BTreeSet<Subject>);

impl SubjectCollection {
    /// Query whether a specific subject is contained in the collection.
    pub fn contains(&self, subject: &Subject) -> bool {
        self.0.contains(subject)
    }

    /// Query whether the subject collection contains an entry for a user.
    pub fn contains_any_user(&self) -> bool {
        self.0.iter().any(Subject::is_user)
    }

    /// Query whether the subject collection contains any of the users present in the keys of a `BTreeMap`.
    pub fn contains_any_user_by_key<V>(&self, users: &BTreeMap<UserId, V>) -> bool {
        self.0
            .iter()
            .filter_map(|s| match s {
                Subject::User(id) => Some(id),
                Subject::InviteCode(_) => None,
            })
            .any(|id| users.contains_key(id))
    }

    /// Query whether the subject collection contains any of the invite codes that are not expired in a `BTreeMap`.
    pub fn contains_any_valid_invite_code(
        &self,
        invites: &BTreeMap<InviteCode, Option<Timestamp>>,
    ) -> bool {
        let now = Timestamp::now();
        self.0
            .iter()
            .filter_map(|s| match s {
                Subject::User(_) => None,
                Subject::InviteCode(code) => Some(code),
            })
            .any(|code| {
                invites
                    .get(code)
                    .is_some_and(|expiration| expiration.is_none_or(|expiration| expiration > now))
            })
    }

    /// Query whether any of the users in the subject collection has a role equal or higher in a
    /// given `BTreeMap` of users with their roles.
    pub fn any_user_has_role_or_higher(
        &self,
        users: &BTreeMap<UserId, InviteRole>,
        minimum_role: InviteRole,
    ) -> bool {
        users.iter().any(|(&user_id, &role)| {
            role >= minimum_role && self.0.contains(&Subject::User(user_id))
        })
    }
}

impl FromIterator<Subject> for SubjectCollection {
    fn from_iter<I: IntoIterator<Item = Subject>>(iter: I) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

#[cfg(feature = "actix-web")]
pub(super) mod actix_web_impls {

    use actix_web::{HttpMessage, dev::ServiceRequest};
    use opentalk_types_common::{rooms::invite_codes::InviteCode, users::UserId};

    use super::*;

    impl From<&ServiceRequest> for SubjectCollection {
        fn from(req: &ServiceRequest) -> Self {
            let maybe_invite_code = req.extensions().get::<InviteCode>().map(Subject::from);
            let maybe_user_id = req.extensions().get::<UserId>().map(Subject::from);

            Self(maybe_invite_code.into_iter().chain(maybe_user_id).collect())
        }
    }
}
