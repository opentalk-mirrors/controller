// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use snafu::Snafu;

/// Error performing synchronization.
#[derive(Debug, Snafu)]
pub enum SynchronizationError {
    /// Synchronization between instances failed
    SynchronizationFailed,
}
