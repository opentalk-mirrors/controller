// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::{fmt::Display, time::Duration};
use std::hash::Hash;

use bincode::{Decode, Encode};
use serde::de::DeserializeOwned;
use snafu::Snafu;

pub mod local;
pub mod redis;

/// Application level cache which can store entries both in a locally and distributed using redis
pub struct Cache<K, V> {
    local: local::Cache<K, V>,
    redis: Option<redis::Cache<K, V>>,
}

#[derive(Debug, Snafu)]
pub enum CacheError {
    #[snafu(display("Redis error: {}", source), context(false))]
    Redis { source: redis::Error },
    #[snafu(display("Encode error: {}", source), context(false))]
    Encode { source: bincode::error::EncodeError },
    #[snafu(display("Decode error: {}", source), context(false))]
    Decode { source: bincode::error::DecodeError },
}

impl<K, V> Cache<K, V>
where
    K: Display + Hash + Eq + Send + Sync + 'static,
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

    /// Return the longest duration an entry might live for
    pub fn longest_ttl(&self) -> Duration {
        let local_ttl = self.local.ttl();

        self.redis
            .as_ref()
            .map(|c| c.ttl().max(local_ttl))
            .unwrap_or(local_ttl)
    }

    pub async fn get(&self, key: &K) -> Result<Option<V>, CacheError> {
        if let Some(value) = self.local.get(key).await {
            Ok(Some(value))
        } else if let Some(r) = &self.redis {
            Ok(r.get(key).await?)
        } else {
            Ok(None)
        }
    }

    /// Insert a key-value pair with the cache's default TTL
    pub async fn insert(&self, key: K, value: V) -> Result<(), CacheError> {
        if let Some(r) = &self.redis {
            r.insert(&key, &value).await?;
        }

        self.local.insert(key, value).await;

        Ok(())
    }

    /// Insert an entry with a custom TTL
    ///
    /// Note that TTLs larger than the configured one will be ignored
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) -> Result<(), CacheError> {
        if ttl >= self.longest_ttl() {
            return self.insert(key, value).await;
        }

        if let Some(r) = &self.redis {
            r.insert_with_ttl(&key, &value, ttl).await?;
        }

        self.local.insert_with_ttl(key, value, ttl).await;

        Ok(())
    }
}
