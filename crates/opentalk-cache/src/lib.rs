// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::time::Duration;

pub use cache_error::CacheError;

mod cache_error;
pub mod local;
pub mod overlay;
pub mod redis;

pub type Result<T, E = CacheError> = std::result::Result<T, E>;

#[async_trait::async_trait(?Send)]
pub trait CacheStorage<K, V>: Sync + Send {
    fn ttl(&self) -> Duration;

    async fn get(&self, key: &K) -> Result<Option<V>>;

    async fn insert(&self, key: K, value: V) -> Result<()>;

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()>;
}
