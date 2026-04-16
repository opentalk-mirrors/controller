// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use snafu::Snafu;

/// Error performing the authorization.
#[derive(Debug, Snafu)]
pub enum AuthorizationError {
    /// Synchronization between instances failed
    SynchronizationFailed,
}
