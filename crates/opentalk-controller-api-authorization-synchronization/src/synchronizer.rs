// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::sync::Arc;

use opentalk_controller_api_authorization::authorization::AuthorizationChange;
use tokio::sync::RwLock;

use crate::{SynchronizationError, SynchronizerBackend};

/// Synchronizes changes in the authorization data by sending and receiving
/// [`AuthorizationChange`] items.
#[derive(Clone)]
pub struct Synchronizer {
    backend: Arc<RwLock<dyn SynchronizerBackend>>,
}

impl std::fmt::Debug for Synchronizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Synchronizer")
    }
}

impl Synchronizer {
    /// Send changes in the authorization data to other nodes
    pub async fn send_changes(
        &self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), SynchronizationError> {
        let backend = self.backend.read().await;
        backend.send_changes(changeset).await
    }

    /// Receive changes in the authorization data from other nodes
    pub async fn receive_changes(&self) -> Vec<AuthorizationChange> {
        let backend = self.backend.read().await;
        backend.receive_changes().await
    }
}
