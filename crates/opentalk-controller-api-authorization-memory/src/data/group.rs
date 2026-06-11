// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use opentalk_types_common::users::UserId;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Group {
    users: BTreeSet<UserId>,
}

impl Group {
    pub fn add_user(&mut self, user: UserId) {
        let _ = self.users.insert(user);
    }

    pub fn remove_user(&mut self, user: &UserId) {
        let _ = self.users.remove(user);
    }
}
