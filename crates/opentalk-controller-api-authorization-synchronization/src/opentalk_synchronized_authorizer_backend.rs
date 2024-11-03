// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::time::Duration;

use async_trait::async_trait;
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
        let (shutdown_sender, shutdown_receiver) = flume::bounded(1);

        let _receiver_loop_join_handle =
            spawn_receiver_loop(upstream.clone(), synchronizer.clone(), shutdown_receiver);

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
        if let Err(e) = self.shutdown_sender.send_timeout((), TIMEOUT) {
            log::warn!(
                "Shutdown signal for authorization synchronization task was not sent within timeout {TIMEOUT:?}: {e}"
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
            log::warn!("Error sending authorization change to other nodes: {e}");
            log::warn!(
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
) -> JoinHandle<()> {
    task::spawn(async move {
        receive_and_apply_changes_loop(upstream, synchronizer, shutdown_receiver).await
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
) {
    while let Continuation::Continue = receive_and_apply_changes(
        upstream.clone(),
        synchronizer.clone(),
        shutdown_receiver.clone(),
    )
    .await
    {}
}

async fn receive_and_apply_changes(
    upstream: Authorizer,
    synchronizer: Synchronizer,
    shutdown_receiver: flume::Receiver<()>,
) -> Continuation {
    select! {
        changes = synchronizer.receive_changes() => {
            println!("applying changes {changes:?}…");
            if let Err(e) = upstream.apply_changes(&changes).await {
                log::warn!("Error applying authorization changes from other node: {e}");
            }
            Continuation::Continue
        }
        _ = shutdown_receiver.recv_async() => {
            println!("shutdown signal received, stopping authorization synchronizer receiver task");
            Continuation::Stop
        }
    }
}
