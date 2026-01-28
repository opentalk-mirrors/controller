// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::time::Duration;
use std::{fmt::Display, hash::Hash};

use chrono::TimeDelta;
use opentalk_cache::{
    CacheError, CacheStorage, hashing::WithHashing, local, overlay::WithOverlay, redis,
};
use opentalk_signaling_core::RedisConnection;
use snafu::Snafu;

use crate::caching::cacheable::AccessTokenResult;

#[derive(Debug, Snafu)]
pub enum AccesTokenCacheError {
    #[snafu(display("cache error: {source}"))]
    Cache { source: CacheError },

    #[snafu(display("token expires soon and will not be cached (ttl={ttl:?})"))]
    TokenTtlTooShort { ttl: TimeDelta },

    #[snafu(display("token has no expiry and will not be cached"))]
    NoExpiryForToken,
}

pub type Result<T, E = AccesTokenCacheError> = std::result::Result<T, E>;

impl From<CacheError> for AccesTokenCacheError {
    fn from(source: CacheError) -> Self {
        Self::Cache { source }
    }
}

/// Cache for OpenID Connect related data
pub struct Cache {
    /// Cache storage for access tokens
    pub access_tokens: Box<dyn CacheStorage<String, AccessTokenResult> + Send + Sync>,
}

impl Cache {
    /// Create a new [`Cache`] instance with an optional [`RedisConnection`].
    pub fn create(redis: Option<RedisConnection>) -> Self {
        Self {
            access_tokens: Self::build_cache(
                redis,
                "user-access-tokens".to_string(),
                Duration::from_secs(300),
            ),
        }
    }

    fn build_cache<K, V>(
        redis: Option<RedisConnection>,
        prefix: String,
        ttl: Duration,
    ) -> Box<dyn CacheStorage<K, V> + Send + Sync>
    where
        K: redis::Key + local::Key + Clone + Display + Hash + 'static,
        V: redis::Value + local::Value + 'static,
    {
        let local_cache = local::Cache::new(ttl);

        let Some(redis) = redis else {
            return Box::new(local_cache.with_hashing());
        };
        let redis = redis.into_manager();

        let redis_cache = redis::Cache::new(redis, prefix, ttl);

        Box::new(redis_cache.with_overlay(local_cache).with_hashing())
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caches")
    }
}
