// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::{fmt::Display, time::Duration};
use std::{hash::Hash, time::Instant};

use bincode::{Decode, Encode};
use moka::future::Cache as LocalCache;
use serde::de::DeserializeOwned;
use snafu::Snafu;

pub mod redis;

/// Application level cache which can store entries both in a locally and distributed using redis
pub struct Cache<K, V> {
    local: LocalCache<K, LocalEntry<V>>,
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
            local: LocalCache::builder().time_to_live(ttl).build(),
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
        let local_ttl = self
            .local
            .policy()
            .time_to_live()
            .expect("local always has a ttl");

        self.redis
            .as_ref()
            .map(|c| c.ttl().max(local_ttl))
            .unwrap_or(local_ttl)
    }

    pub async fn get(&self, key: &K) -> Result<Option<V>, CacheError> {
        if let Some(entry) = self
            .local
            .get(key)
            .await
            .filter(|entry| entry.still_valid())
        {
            Ok(Some(entry.value))
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

        self.local
            .insert(
                key,
                LocalEntry {
                    value,
                    expires_at: None,
                },
            )
            .await;

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

        self.local
            .insert(
                key,
                LocalEntry {
                    value,
                    expires_at: Some(Instant::now() + ttl),
                },
            )
            .await;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalEntry<V> {
    value: V,
    /// Custom expiration value to work around moka's limitation to set a custom ttl for an entry
    expires_at: Option<Instant>,
}

impl<V> LocalEntry<V> {
    // Check if the custom ttl has expired
    fn still_valid(&self) -> bool {
        if let Some(exp) = self.expires_at {
            exp.saturating_duration_since(Instant::now()) > Duration::ZERO
        } else {
            true
        }
    }
}
