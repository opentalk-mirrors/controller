// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{rooms::invite_codes::InviteCode, users::UserId};

/// A subject for which the access to resources can be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// An authenticated user.
    User(UserId),

    /// An invite code.
    InviteCode(InviteCode),
}
