// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::{fmt::Display, time::Duration};
use std::hash::Hash;

use bincode::{Decode, Encode};
pub use cache_error::CacheError;
use serde::de::DeserializeOwned;

mod cache_error;
pub mod local;
pub mod redis;

pub type Result<T, E = CacheError> = std::result::Result<T, E>;

#[async_trait::async_trait(?Send)]
pub trait CacheStorage<K, V>: Sync + Send {
    fn ttl(&self) -> Duration;

    async fn get(&self, key: &K) -> Result<Option<V>>;

    async fn insert(&self, key: K, value: V) -> Result<()>;

    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()>;
}

/// Application level cache which can store entries both in a locally and distributed using redis
pub struct Cache<K, V> {
    local: local::Cache<K, V>,
    redis: Option<redis::Cache<K, V>>,
}

impl<K, V> Cache<K, V>
where
    K: Display + Hash + Eq + Clone + Send + Sync + 'static,
    V: Encode + Decode<()> + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            local: local::Cache::new(ttl),
            redis: None,
        }
    }

    pub fn with_redis(
        self,
        connection: redis::Connection,
        prefix: impl Into<String>,
        ttl: Duration,
        hash_key: bool,
    ) -> Self {
        Self {
            redis: Some(redis::Cache::new(connection, prefix.into(), ttl, hash_key)),
            ..self
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<K, V> CacheStorage<K, V> for Cache<K, V>
where
    K: Display + Hash + Eq + Clone + Send + Sync + 'static,
    V: Encode + Decode<()> + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn ttl(&self) -> Duration {
        let local_ttl = self.local.ttl();

        self.redis
            .as_ref()
            .map(|c| c.ttl().max(local_ttl))
            .unwrap_or(local_ttl)
    }

    async fn get(&self, key: &K) -> Result<Option<V>> {
        if let Some(value) = self.local.get(key).await.unwrap() {
            Ok(Some(value))
        } else if let Some(r) = &self.redis {
            Ok(r.get(key).await?)
        } else {
            Ok(None)
        }
    }

    async fn insert(&self, key: K, value: V) -> Result<()> {
        if let Some(r) = &self.redis {
            r.insert(key.clone(), value.clone()).await?;
        }

        self.local.insert(key, value).await
    }

    /// Insert an entry with a custom TTL
    ///
    /// Note that TTLs larger than the configured one will be ignored
    async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<()> {
        if ttl >= self.ttl() {
            return self.insert(key, value).await;
        }

        if let Some(r) = &self.redis {
            r.insert_with_ttl(key.clone(), value.clone(), ttl).await?;
        }

        self.local.insert_with_ttl(key, value, ttl).await
    }
}
