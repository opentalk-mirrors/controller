// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::sync::Arc;

use opentalk_controller_api_authorization::authorization::AuthorizationChange;

use crate::{SynchronizationError, SynchronizerBackend};

/// Synchronizes changes in the authorization data by sending and receiving
/// [`AuthorizationChange`] items.
#[derive(Clone)]
pub struct Synchronizer {
    backend: Arc<dyn SynchronizerBackend>,
}

impl std::fmt::Debug for Synchronizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Synchronizer")
    }
}

impl Synchronizer {
    /// Create a new synchronizer
    pub fn new<B: SynchronizerBackend + 'static>(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Send changes in the authorization data to other nodes
    pub async fn send_changes(
        &self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), SynchronizationError> {
        self.backend.send_changes(changeset).await
    }

    /// Receive changes in the authorization data from other nodes
    pub async fn receive_changes(&self) -> Option<Vec<AuthorizationChange>> {
        self.backend.receive_changes().await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use opentalk_controller_api_authorization::authorization::AuthorizationChange;
    use opentalk_types_common::{
        events::EventId,
        users::{GroupId, UserId},
    };
    use pretty_assertions::assert_eq;

    use super::Synchronizer;
    use crate::synchronizer_backend::MockSynchronizerBackend;

    fn build_changeset() -> Vec<AuthorizationChange> {
        vec![
            AuthorizationChange::AddUserToGroups {
                user: UserId::from_u128(0x12345678),
                groups: BTreeSet::from_iter([
                    GroupId::from_u128(0x8888),
                    GroupId::from_u128(0x9999),
                ]),
            },
            AuthorizationChange::DeleteEvent {
                event: EventId::from_u128(0x1337),
            },
        ]
    }

    #[tokio::test]
    async fn send_changes() {
        let mut backend = MockSynchronizerBackend::new();
        let _ = backend
            .expect_send_changes()
            .once()
            .withf(move |c| c == build_changeset().as_slice())
            .returning(|_v| Ok(()));

        let synchronizer = Synchronizer::new(backend);

        synchronizer
            .send_changes(&build_changeset())
            .await
            .expect("Changes should be sent");
    }

    #[tokio::test]
    async fn receive_changes() {
        let mut backend = MockSynchronizerBackend::new();
        let _ = backend
            .expect_receive_changes()
            .once()
            .returning(|| Some(build_changeset()));

        let synchronizer = Synchronizer::new(backend);

        assert_eq!(
            synchronizer.receive_changes().await,
            Some(build_changeset())
        );
    }
}
