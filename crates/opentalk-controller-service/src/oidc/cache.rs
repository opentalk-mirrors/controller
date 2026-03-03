// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::time::Duration;
use std::{fmt::Display, hash::Hash};

use chrono::{DateTime, TimeDelta, Utc};
use openidconnect::{AccessToken, SubjectIdentifier};
use opentalk_cache::{
    CacheError, CacheStorage, CacheUpdateMode, hashing::WithHashing, local, overlay::WithOverlay,
    redis,
};
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Tenant, User};
use opentalk_signaling_core::RedisConnection;
use snafu::{Report, Snafu};

use super::{
    LogoutMarker,
    cacheable::{
        AccessTokenResult, ApiError as CacheableApiError, DecodeFromCacheError,
        Tenant as CacheableTenant, User as CacheableUser,
    },
};

#[derive(Debug, Snafu)]
pub enum OidcCacheError {
    #[snafu(display("cache error: {source}"))]
    Cache { source: CacheError },

    #[snafu(display("cache error while decoding: {source}"))]
    DecodeFromCacheError { source: DecodeFromCacheError },

    #[snafu(display("token expires soon and will not be cached (ttl={ttl:?})"))]
    TokenTtlTooShort { ttl: TimeDelta },

    #[snafu(display("token has no expiry and will not be cached"))]
    NoExpiryForToken,

    #[snafu(display("token doen't exist in cache and cannot be updated"))]
    CannotUpdateNonExistingToken,
}

pub type Result<T, E = OidcCacheError> = std::result::Result<T, E>;

impl From<CacheError> for OidcCacheError {
    fn from(source: CacheError) -> Self {
        Self::Cache { source }
    }
}

impl From<DecodeFromCacheError> for OidcCacheError {
    fn from(source: DecodeFromCacheError) -> Self {
        Self::DecodeFromCacheError { source }
    }
}

const ACCESS_TOKEN_DEFAULT_TTL_SECS: u64 = 60 * 5;
/// Must be much longer, than for access tokens
const SUB_LOGOUT_MARKERS_DEFAULT_TTL_SECS: u64 = 60 * 60 * 2;
const MIN_TOKEN_TTL_SECS: i64 = 10;

/// Cache for OpenID Connect related data
pub struct Cache {
    /// Cache storage for access tokens
    pub access_tokens: Box<dyn CacheStorage<String, AccessTokenResult> + Send + Sync>,
    /// Cache storage for logout markers of the OIDC subjects
    pub sub_logout_markers: Box<dyn CacheStorage<String, LogoutMarker> + Send + Sync>,
}

impl Cache {
    /// Create a new [`Cache`] instance with an optional [`RedisConnection`].
    pub fn create(redis: Option<RedisConnection>) -> Self {
        Self {
            access_tokens: Self::build_cache_with_hashing(
                redis.clone(),
                "user-access-tokens".to_string(),
                Duration::from_secs(ACCESS_TOKEN_DEFAULT_TTL_SECS),
                CacheUpdateMode::KeepTtl,
            ),
            sub_logout_markers: Self::build_cache(
                redis,
                "sub-logout-markers".to_string(),
                Duration::from_secs(SUB_LOGOUT_MARKERS_DEFAULT_TTL_SECS),
                CacheUpdateMode::ResetTtl,
            ),
        }
    }

    /// Build a cache storage
    /// If Redis connection is available, combine a Redis cache with a local in-memory cache as an overlay
    /// Otherise only local cache is used
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
            return Box::new(local_cache);
        };
        let redis = redis.into_manager();

        let redis_cache = redis::Cache::new(redis, prefix, ttl, mode);

        Box::new(redis_cache.with_overlay(local_cache))
    }

    /// Build a cache storage with key hashing to reduce memory usage for large keys
    /// If Redis connection is available, compose a Redis cache with a local in-memory cache as an overly
    /// Otherise only local cache is used
    fn build_cache_with_hashing<K, V>(
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
            return Err(OidcCacheError::NoExpiryForToken);
        };

        let token_ttl = expires_at - Utc::now();

        // Don't cache tokens that expire too soon
        if token_ttl <= chrono::Duration::seconds(MIN_TOKEN_TTL_SECS) {
            return Err(OidcCacheError::TokenTtlTooShort { ttl: token_ttl });
        }

        let value = match value {
            Ok((tenant, user)) => {
                let logout_marker = self.calculate_logout_marker(&user.oidc_sub).await?;
                Ok((
                    CacheableTenant::from(tenant),
                    CacheableUser::from(user),
                    logout_marker,
                ))
            }
            Err(e) => Err(CacheableApiError::from(e)),
        };

        self.access_tokens
            .insert_with_ttl(
                access_token.secret().clone(),
                value,
                token_ttl.to_std().expect("duration was previously checked"),
            )
            .await
            .map_err(OidcCacheError::from)
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

        let cached = match self.access_tokens.get(access_token.secret()).await {
            Ok(Some(result)) => result,
            Ok(None) => return Err(OidcCacheError::CannotUpdateNonExistingToken),
            Err(e) => return Err(OidcCacheError::from(e)),
        };

        // For valid token we need to preserve the logout marker
        let value_with_logout = match cached {
            Ok((_, _, logout_marker)) => value.map(|(tenant, user)| (tenant, user, logout_marker)),
            Err(e) => Err(e),
        };

        // We know the token exists in the cache, so we can safely update it with default TTL
        // The cache is configured to keep original TTL on update
        self.access_tokens
            .insert(access_token.secret().clone(), value_with_logout)
            .await
            .map_err(OidcCacheError::from)
    }

    /// Get cached result for an access token
    pub async fn get_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Result<Option<Result<(Tenant, User), CaptureApiError>>, OidcCacheError> {
        match self.access_tokens.get(access_token.secret()).await {
            Ok(Some(Ok((tenant, user, _)))) => {
                let tenant = tenant.try_into().map_err(|e| {
                log::warn!(
                    "Error when attempting to read tenant information loaded from token cache: {}",
                    Report::from_error(&e)
                );
                OidcCacheError::from(e)
            })?;
                let user = user.try_into().map_err(|e| {
                log::warn!(
                    "Error when attempting to read user information loaded from token cache: {}",
                    Report::from_error(&e)
                );
                OidcCacheError::from(e)
            })?;

                Ok(Some(Ok((tenant, user))))
            }
            Ok(Some(Err(cached_error))) => {
                let cached_error = CaptureApiError::try_from(cached_error).map_err(|e| {
                log::warn!(
                    "Error when attempting to read verification result loaded from token cache: {}",
                    Report::from_error(&e)
                );
                OidcCacheError::from(e)
            })?;
                Ok(Some(Err(cached_error)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Insert logout marker for a specific OIDC subject
    async fn insert_sub_logout_marker(&self, sub: String, marker: LogoutMarker) -> Result<()> {
        self.sub_logout_markers
            .insert(sub, marker)
            .await
            .map_err(|e| {
                log::warn!(
                    "Failed to cache logout marker, error: {}",
                    Report::from_error(&e)
                );
                OidcCacheError::from(e)
            })
    }

    /// Insert or update logout marker for a specific OIDC subject in the cache
    /// to invalidate all existing access tokens for the subject
    pub async fn upsert_sub_logout_marker(&self, sub: SubjectIdentifier) -> Result<()> {
        let sub = String::from(sub);
        let logout_marker = self.calculate_logout_marker(&sub).await?;
        self.insert_sub_logout_marker(sub, logout_marker).await?;
        Ok(())
    }

    /// Calculate logout marker for the associated OIDC subject
    /// If marker for the subject does not exist: insert a new one with value 0
    /// If marker for the subject exists: increment the marker
    async fn calculate_logout_marker(&self, sub: &String) -> Result<LogoutMarker> {
        match self.sub_logout_markers.get(sub).await {
            Ok(Some(mut marker)) => {
                marker.increment();
                Ok(marker)
            }
            Ok(None) => Ok(LogoutMarker::from(0)),
            Err(e) => {
                log::warn!(
                    "Failed to retreive logout marker from the cache, error: {}",
                    Report::from_error(&e)
                );
                Err(OidcCacheError::from(e))
            }
        }
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caches")
    }
}
