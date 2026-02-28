// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::time::Duration;
use std::{fmt::Display, hash::Hash};

use chrono::{DateTime, TimeDelta, Utc};
use openidconnect::AccessToken;
use opentalk_cache::{
    CacheError, CacheStorage, CacheUpdateMode, hashing::WithHashing, local, overlay::WithOverlay,
    redis,
};
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Tenant, User};
use opentalk_signaling_core::RedisConnection;
use snafu::Snafu;

use super::cacheable::{
    AccessTokenResult, ApiError as CacheableApiError, Tenant as CacheableTenant,
    User as CacheableUser,
};

#[derive(Debug, Snafu)]
pub enum AccesTokenCacheError {
    #[snafu(display("cache error: {source}"))]
    Cache { source: CacheError },

    #[snafu(display("token expires soon and will not be cached (ttl={ttl:?})"))]
    TokenTtlTooShort { ttl: TimeDelta },

    #[snafu(display("token has no expiry and will not be cached"))]
    NoExpiryForToken,

    #[snafu(display("token doen't exist in cache and cannot be updated"))]
    CannotUpdateNonExistingToken,
}

pub type Result<T, E = AccesTokenCacheError> = std::result::Result<T, E>;

impl From<CacheError> for AccesTokenCacheError {
    fn from(source: CacheError) -> Self {
        Self::Cache { source }
    }
}

const MIN_TOKEN_TTL_SECS: i64 = 10;

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
                CacheUpdateMode::KeepTtl,
            ),
        }
    }

    fn build_cache<K, V>(
        redis: Option<RedisConnection>,
        prefix: String,
        ttl: Duration,
        mode: CacheUpdateMode,
    ) -> Box<dyn CacheStorage<K, V> + Send + Sync>
    where
        K: redis::Key + local::Key + Clone + Display + Hash + 'static,
        V: redis::Value + local::Value + 'static,
    {
        let local_cache = local::Cache::new(ttl, mode);

        let Some(redis) = redis else {
            return Box::new(local_cache.with_hashing());
        };
        let redis = redis.into_manager();

        let redis_cache = redis::Cache::new(redis, prefix, ttl, mode);

        Box::new(redis_cache.with_overlay(local_cache).with_hashing())
    }

    /// Insert an access token into the cache with a specific expiry date
    /// Cache will reject a token, which has no expiry date or its ttl is too short
    /// Value is either a valid token metadata or an error
    pub async fn insert_access_token(
        &self,
        access_token: &AccessToken,
        value: Result<(Tenant, User), CaptureApiError>,
        maybe_expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let Some(expires_at) = maybe_expires_at else {
            return Err(AccesTokenCacheError::NoExpiryForToken);
        };

        let token_ttl = expires_at - Utc::now();

        // Don't cache tokens that expire too soon
        if token_ttl <= chrono::Duration::seconds(MIN_TOKEN_TTL_SECS) {
            return Err(AccesTokenCacheError::TokenTtlTooShort { ttl: token_ttl });
        }

        let value = match value {
            Ok((tenant, user)) => Ok((CacheableTenant::from(tenant), CacheableUser::from(user))),
            Err(e) => Err(CacheableApiError::from(e)),
        };

        self.access_tokens
            .insert_with_ttl(
                access_token.secret().clone(),
                value,
                token_ttl.to_std().expect("duration was previously checked"),
            )
            .await
            .map_err(AccesTokenCacheError::from)
    }

    /// Updates value for a valid cached access token
    /// Cache will reject a token, which do not exist in the cache yet
    /// On update the original TTL of the cache entry will be kept
    /// Value is either a valid token metadata or an error
    pub async fn update_access_token(
        &self,
        access_token: &AccessToken,
        value: Result<(Tenant, User), CaptureApiError>,
    ) -> Result<()> {
        let value = match value {
            Ok((tenant, user)) => Ok((CacheableTenant::from(tenant), CacheableUser::from(user))),
            Err(e) => Err(CacheableApiError::from(e)),
        };

        match self.access_tokens.get(access_token.secret()).await {
            Ok(Some(_)) => (),
            Ok(None) => return Err(AccesTokenCacheError::CannotUpdateNonExistingToken),
            Err(e) => return Err(AccesTokenCacheError::from(e)),
        }

        // We know the token exists in the cache, so we can safely update it with default TTL
        // The cache is configured to keep original TTL on update
        self.access_tokens
            .insert(access_token.secret().clone(), value)
            .await
            .map_err(AccesTokenCacheError::from)
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caches")
    }
}
