// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use async_trait::async_trait;
use opentalk_controller_api_authorization::authorization::AuthorizationChange;

use crate::SynchronizationError;

/// A trait for implementing synchronization backends for the OpenTalk
/// controller api authorization
#[async_trait]
pub trait SynchronizerBackend: Send + Sync {
    /// Send changes in the authorization data to other nodes
    async fn send_changes(
        &self,
        _changeset: &[AuthorizationChange],
    ) -> Result<(), SynchronizationError>;

    /// Receive changes in the authorization data from other nodes
    async fn receive_changes(&self) -> Vec<AuthorizationChange>;
}
