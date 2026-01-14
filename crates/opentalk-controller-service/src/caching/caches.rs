// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::time::Duration;

use opentalk_cache::{CacheStorage, local, overlay::WithOverlay, redis};
use opentalk_signaling_core::RedisConnection;

use super::UserAccessTokenResult;

/// Holds all application level caches
pub struct Caches {
    /// Cache the results of user access-token checks
    pub user_access_tokens: Box<dyn CacheStorage<String, UserAccessTokenResult> + Send + Sync>,
}

impl Caches {
    /// Create a new [`Caches`] instance with an optional [`RedisConnection`].
    pub fn create(redis: Option<RedisConnection>) -> Self {
        Self {
            user_access_tokens: Self::build_cache(
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
        K: redis::Key + local::Key + Clone + 'static,
        V: redis::Value + local::Value + 'static,
    {
        let local_cache = local::Cache::new(ttl);

        let Some(redis) = redis else {
            return Box::new(local_cache);
        };
        let redis = redis.into_manager();

        let redis_cache = redis::Cache::new(redis, prefix, ttl, true);

        Box::new(redis_cache.with_overlay(local_cache))
    }
}

impl std::fmt::Debug for Caches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caches")
    }
}
