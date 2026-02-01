// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};

/// Info returned from the access token introspection
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct IntrospectInfo {
    /// Access token is still active
    pub active: bool,
    /// Expire timestamp of the token
    pub exp: Option<DateTime<Utc>>,
    /// Subject of the access token
    pub sub: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntrospectStrippedInfo {
    /// Expire timestamp of the token
    pub exp: Option<DateTime<Utc>>,
    /// Subject of the access token
    pub sub: Option<String>,
}

impl From<IntrospectInfo> for IntrospectStrippedInfo {
    fn from(info: IntrospectInfo) -> Self {
        Self {
            exp: info.exp,
            sub: info.sub,
        }
    }
}
