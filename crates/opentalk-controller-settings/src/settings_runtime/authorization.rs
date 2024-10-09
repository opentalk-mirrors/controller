// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use crate::settings_file;

/// Authorization settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    /// Synchronize authorization data between controllers if true.
    pub synchronize_controllers: bool,
}

impl Authorization {
    pub(crate) fn from_settings_file(
        value: Option<settings_file::Authorization>,
        is_rabbitmq_available: bool,
    ) -> Self {
        let synchronize_controllers = is_rabbitmq_available
            && value
                .and_then(|v| v.synchronize_controllers)
                .unwrap_or(true);
        Self {
            synchronize_controllers,
        }
    }
}
