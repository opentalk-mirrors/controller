// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use async_trait::async_trait;

use super::{
    Admission, AuthorizationChange, AuthorizationChangeError, AuthorizationError,
    AuthorizationTarget,
};

/// A trait for implementing authorization queries against the OpenTalk Controller API.
#[async_trait]
pub trait AuthorizerBackend: Send + Sync {
    /// Attempt to authorize for a specific resource as a subject.
    async fn authorize(&self, target: AuthorizationTarget)
    -> Result<Admission, AuthorizationError>;

    /// Apply a changeset to the authorization backend data
    async fn apply_changes(
        &mut self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), AuthorizationChangeError>;
}
