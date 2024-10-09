// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// Outcome of an authorization check.
#[derive(Debug)]
pub enum Admission {
    /// Admission to the requested resource is allowed.
    Allowed,

    /// Admission to the requested resource is denied.
    Denied,
}
