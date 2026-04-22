// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileCopyrightText: OpenTalk Team <mail@opentalk.eu>

use async_trait::async_trait;
use opentalk_roomserver_client::Client;
use opentalk_types_api_internal::module_assets::Quota;
use opentalk_types_common::users::UserId;

#[async_trait]
pub trait StorageNotifier: Send + Sync {
    async fn notify(&self, user_id: UserId, old_quota: Quota, new_quota: Quota);
}

#[derive(Debug, Clone)]
pub struct RoomServerStorageNotifier {
    client: Client,
}

impl RoomServerStorageNotifier {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl StorageNotifier for RoomServerStorageNotifier {
    async fn notify(&self, user_id: UserId, _old_quota: Quota, new_quota: Quota) {
        if let Err(err) = self.client.post_storage_quota(user_id, new_quota).await {
            tracing::error!("Failed to post roomserver storage quota: {err}");
        } else {
            log::debug!("Notified RoomServer about quota change");
        }
    }
}

#[async_trait]
impl<T> StorageNotifier for Option<T>
where
    T: StorageNotifier + Sync,
{
    async fn notify(&self, user_id: UserId, old_quota: Quota, new_quota: Quota) {
        if let Some(notifier) = self {
            notifier.notify(user_id, old_quota, new_quota).await;
        }
    }
}

pub struct NoOpStorageNotifier;

#[async_trait]
impl StorageNotifier for NoOpStorageNotifier {
    async fn notify(&self, user_id: UserId, old_quota: Quota, new_quota: Quota) {
        log::debug!(
            "Skipped storage notification for user: {user_id}. Quota update from {old_quota:?} to {new_quota:?}"
        );
    }
}
