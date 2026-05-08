// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileCopyrightText: OpenTalk Team <mail@opentalk.eu>

//! This module contains the implementation of a [`StorageNotifier`] which notifies the RoomServer about changes in the storage quota.

use std::sync::Arc;

use opentalk_asset_storage::{NoOpStorageNotifier, StorageNotifier};
use opentalk_controller_settings::RoomServerKind;
use opentalk_roomserver_client::Client;
use opentalk_types_api_internal::module_assets::Quota;
use opentalk_types_common::users::UserId;

/// Creates a [`StorageNotifier`] instance for the provided [`RoomServerKind`].
pub fn build_storage_notifier(kind: &RoomServerKind) -> Arc<dyn StorageNotifier> {
    match kind {
        // TODO: replace once a storage notifier for the internal roomserver has been implemented
        RoomServerKind::Internal { .. } => Arc::new(NoOpStorageNotifier),
        RoomServerKind::External {
            service_url,
            api_key,
        } => Arc::new(ExternalStorageNotifier::new(Client::new(
            service_url.clone(),
            api_key.clone(),
        ))),
    }
}

#[derive(Debug, Clone)]
/// A storage notifier for an external roomserver.
pub struct ExternalStorageNotifier {
    client: Client,
}

impl ExternalStorageNotifier {
    /// Creates a new [`RoomServerStorageNotifier`]
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl StorageNotifier for ExternalStorageNotifier {
    async fn notify(&self, user_id: UserId, _old_quota: Quota, new_quota: Quota) {
        if let Err(err) = self.client.post_storage_quota(user_id, new_quota).await {
            log::error!("Failed to post roomserver storage quota: {err}");
        } else {
            log::debug!("Notified RoomServer about quota change");
        }
    }
}
