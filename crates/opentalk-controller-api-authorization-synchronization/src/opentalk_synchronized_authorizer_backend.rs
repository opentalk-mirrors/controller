// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::Duration;

use async_trait::async_trait;
use flume::SendTimeoutError;
use opentalk_controller_api_authorization::authorization::{
    Admission, AuthorizationChange, AuthorizationChangeError, AuthorizationError,
    AuthorizationTarget, Authorizer, AuthorizerBackend,
};
use tokio::{
    select,
    task::{self, JoinHandle},
};

use crate::synchronizer::Synchronizer;

/// An [`AuthorizerBackend`] which maintains an upstream [`Authorizer`], and
/// synchronizes with other nodes through a [`Synchronizer`].
#[derive(Debug)]
pub struct OpenTalkSynchronizedAuthorizerBackend {
    upstream: Authorizer,
    synchronizer: Synchronizer,
    shutdown_sender: flume::Sender<()>,
}

impl OpenTalkSynchronizedAuthorizerBackend {
    /// Create a new [`OpenTalkSynchronizedAuthorizerBackend`].
    ///
    /// The newly created [`OpenTalkSynchronizedAuthorizerBackend`] will listen
    /// for updates received through the [`Synchronizer`], and will update the
    /// data in the upstream [`Authorizer`] accordingly.
    pub fn new(upstream: Authorizer, synchronizer: Synchronizer) -> Self {
        Self::new_with_is_finished_sender(upstream, synchronizer, None)
    }

    fn new_with_is_finished_sender(
        upstream: Authorizer,
        synchronizer: Synchronizer,
        is_finished_sender: Option<flume::Sender<()>>,
    ) -> Self {
        let (shutdown_sender, shutdown_receiver) = flume::bounded(1);

        let _receiver_loop_join_handle = spawn_receiver_loop(
            upstream.clone(),
            synchronizer.clone(),
            shutdown_receiver,
            is_finished_sender,
        );

        Self {
            upstream,
            synchronizer,
            shutdown_sender,
        }
    }
}

impl Drop for OpenTalkSynchronizedAuthorizerBackend {
    fn drop(&mut self) {
        const TIMEOUT: Duration = Duration::from_millis(500);
        if let Err(SendTimeoutError::Timeout(_)) = self.shutdown_sender.send_timeout((), TIMEOUT) {
            tracing::warn!(
                "Shutdown signal for authorization synchronization task was not sent within timeout {TIMEOUT:?}"
            );
        }
    }
}

#[async_trait]
impl AuthorizerBackend for OpenTalkSynchronizedAuthorizerBackend {
    async fn authorize(
        &self,
        authorization_target: AuthorizationTarget,
    ) -> Result<Admission, AuthorizationError> {
        self.upstream.authorize(authorization_target).await
    }

    async fn apply_changes(
        &mut self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), AuthorizationChangeError> {
        if let Err(e) = self.synchronizer.send_changes(changeset).await {
            tracing::warn!("Error sending authorization change to other nodes: {e}");
            tracing::warn!(
                "Authorization information on the other nodes may be inaccurate until reloaded from the database"
            );
        }
        self.upstream.apply_changes(changeset).await
    }
}

fn spawn_receiver_loop(
    upstream: Authorizer,
    synchronizer: Synchronizer,
    shutdown_receiver: flume::Receiver<()>,
    is_finished_sender: Option<flume::Sender<()>>,
) -> JoinHandle<()> {
    task::spawn(async move {
        receive_and_apply_changes_loop(
            upstream,
            synchronizer,
            shutdown_receiver,
            is_finished_sender,
        )
        .await
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Continuation {
    Continue,
    Stop,
}

async fn receive_and_apply_changes_loop(
    upstream: Authorizer,
    synchronizer: Synchronizer,
    shutdown_receiver: flume::Receiver<()>,
    is_finished_sender: Option<flume::Sender<()>>,
) {
    while let Continuation::Continue = receive_and_apply_changes(
        upstream.clone(),
        synchronizer.clone(),
        shutdown_receiver.clone(),
    )
    .await
    {}
    if let Some(is_finished_sender) = is_finished_sender {
        let _ = is_finished_sender.send_async(()).await;
    }
}

async fn receive_and_apply_changes(
    upstream: Authorizer,
    synchronizer: Synchronizer,
    shutdown_receiver: flume::Receiver<()>,
) -> Continuation {
    select! {
        changes = synchronizer.receive_changes() => {
            match changes {
                Some(changes) => {
                    tracing::debug!("Received authorization changes, applying");
                    if let Err(e) = upstream.apply_changes(&changes).await {
                        tracing::warn!("Error applying authorization changes from other node: {e}");
                    }
                    Continuation::Continue
                }
                None => {
                    tracing::debug!("Authorization synchronizer was shut down, stopping synchronization.");
                    Continuation::Stop
                },
            }
        }
        shutdown = shutdown_receiver.recv_async() => {
            match shutdown{
                Ok(()) => {
                    tracing::debug!("Shutdown signal received, stopping authorization synchronizer receiver task");
                    Continuation::Stop
                }
                Err(e) => {
                    tracing::warn!("Failed to shut down authorization synchronizer receiver task: {e}");
                    Continuation::Stop
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, time::Duration};

    use mockall::{Sequence, predicate::eq};
    use opentalk_controller_api_authorization::authorization::{
        AccessMethod, Admission, AuthorizationChange, AuthorizationError, AuthorizationTarget,
        Authorizer, AuthorizerBackend as _, Resource, Subject, SubjectCollection,
    };
    use opentalk_types_common::{
        events::EventId,
        rooms::RoomId,
        users::{GroupId, UserId},
    };
    use pretty_assertions::assert_eq;
    use tokio::runtime::Runtime;

    use super::OpenTalkSynchronizedAuthorizerBackend;
    use crate::{Synchronizer, synchronizer_backend::MockSynchronizerBackend};

    fn build_authorization_target_a() -> AuthorizationTarget {
        AuthorizationTarget {
            authenticated_subjects: SubjectCollection::from_iter([Subject::User(
                UserId::from_u128(0x1337),
            )]),
            resource: Resource::Events,
            access_method: AccessMethod::Post,
        }
    }

    fn build_authorization_target_b() -> AuthorizationTarget {
        AuthorizationTarget {
            authenticated_subjects: SubjectCollection::from_iter([Subject::User(
                UserId::from_u128(0x9876ff),
            )]),
            resource: Resource::Room(RoomId::from_u128(0x9966)),
            access_method: AccessMethod::Get,
        }
    }

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

    #[tokio::test(flavor = "multi_thread")]
    async fn authorize() {
        let mut upstream =
            opentalk_controller_api_authorization::authorization::MockAuthorizerBackend::new();

        let mut seq = Sequence::new();

        let _ = upstream
            .expect_authorize()
            .once()
            .in_sequence(&mut seq)
            .with(eq(build_authorization_target_a()))
            .returning(|_| Ok(Admission::Denied));

        let _ = upstream
            .expect_authorize()
            .once()
            .in_sequence(&mut seq)
            .with(eq(build_authorization_target_b()))
            .returning(|_| Ok(Admission::Allowed));

        let _ = upstream
            .expect_authorize()
            .once()
            .in_sequence(&mut seq)
            .with(eq(build_authorization_target_a()))
            .returning(|_| Err(AuthorizationError::SynchronizationFailed));

        let synchronizer = MockSynchronizerBackend::new();

        let upstream = Authorizer::new(upstream);
        let synchronizer = Synchronizer::new(synchronizer);
        let backend = OpenTalkSynchronizedAuthorizerBackend::new(upstream, synchronizer);

        assert_eq!(
            backend
                .authorize(build_authorization_target_a())
                .await
                .unwrap(),
            Admission::Denied
        );
        assert_eq!(
            backend
                .authorize(build_authorization_target_b())
                .await
                .unwrap(),
            Admission::Allowed
        );
        assert!(matches!(
            backend.authorize(build_authorization_target_a()).await,
            Err(AuthorizationError::SynchronizationFailed)
        ));
    }

    #[test]
    fn receive_changes_blocking() {
        let runtime = Runtime::new().expect("Runtim expected");
        runtime.block_on(async move {
            let (is_finished_sender, is_finished_receiver) = flume::bounded(1);

            let mut upstream =
                opentalk_controller_api_authorization::authorization::MockAuthorizerBackend::new();

            let mut seq = Sequence::new();

            let mut synchronizer = MockSynchronizerBackend::new();

            let _ = synchronizer
                .expect_receive_changes()
                .once()
                .in_sequence(&mut seq)
                .return_once(|| Some(build_changeset()));

            let _ = upstream
                .expect_apply_changes()
                .once()
                .in_sequence(&mut seq)
                .with(eq(build_changeset()))
                .return_once(|_| Ok(()));
            let _ = synchronizer.expect_receive_changes().return_once(|| None);

            let upstream = Authorizer::new(upstream);
            let synchronizer = Synchronizer::new(synchronizer);
            let backend = OpenTalkSynchronizedAuthorizerBackend::new_with_is_finished_sender(
                upstream,
                synchronizer,
                Some(is_finished_sender),
            );

            assert!(is_finished_receiver.recv_async().await.is_ok());

            drop(backend);
        });
    }

    #[tokio::test]
    async fn receive_changes() {
        let mut upstream =
            opentalk_controller_api_authorization::authorization::MockAuthorizerBackend::new();

        let mut synchronizer = MockSynchronizerBackend::new();

        let mut seq = Sequence::new();

        let _ = synchronizer
            .expect_receive_changes()
            .once()
            .in_sequence(&mut seq)
            .returning(|| Some(build_changeset()));

        let _ = upstream
            .expect_apply_changes()
            .once()
            .in_sequence(&mut seq)
            .with(eq(build_changeset()))
            .returning(|_| Ok(()));

        let _ = synchronizer
            .expect_receive_changes()
            .once()
            .in_sequence(&mut seq)
            .returning(|| None);

        let _ = upstream
            .expect_authorize()
            .once()
            .in_sequence(&mut seq)
            .with(eq(build_authorization_target_a()))
            .returning(|_| Ok(Admission::Denied));

        let upstream = Authorizer::new(upstream);
        let synchronizer = Synchronizer::new(synchronizer);
        let backend = OpenTalkSynchronizedAuthorizerBackend::new(upstream, synchronizer);

        // Give the task some time to call the synchronization
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(
            backend
                .authorize(build_authorization_target_a())
                .await
                .unwrap(),
            Admission::Denied
        );
    }
}
