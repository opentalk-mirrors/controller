// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::sync::Arc;

use tokio::sync::RwLock;

use super::{
    Admission, AuthorizationChange, AuthorizationChangeError, AuthorizationError,
    AuthorizationTarget, AuthorizerBackend,
};

/// A handle holding an authorizer. Contains a thread-safe reference to a
/// `dyn` [`AuthorizerBackend`] implementation locked behind synchronization
/// primitives.
#[derive(Clone)]
pub struct Authorizer {
    backend: Arc<RwLock<dyn AuthorizerBackend>>,
}

impl std::fmt::Debug for Authorizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authorizator")
    }
}

impl Authorizer {
    /// Create a new authorizer
    pub fn new<B: AuthorizerBackend + 'static>(backend: B) -> Self {
        Self {
            backend: Arc::new(RwLock::new(backend)),
        }
    }

    /// Request authorization for a specific [`AuthorizationTarget`].
    pub async fn authorize(
        &self,
        target: AuthorizationTarget,
    ) -> Result<Admission, AuthorizationError> {
        let authorizer = self.backend.read().await;
        authorizer.authorize(target).await
    }

    /// Apply a changeset to the authorization backend data
    pub async fn apply_changes(
        &self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), AuthorizationChangeError> {
        let mut authorizer = self.backend.write().await;
        authorizer.apply_changes(changeset).await
    }

    /// Apply a change to the authorization backend data
    pub async fn apply_change(
        &self,
        change: &AuthorizationChange,
    ) -> Result<(), AuthorizationChangeError> {
        self.apply_changes(std::slice::from_ref(change)).await
    }
}
