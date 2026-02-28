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
use snafu::{Report, ResultExt, Snafu, Whatever};

use super::{
    OnlyExpiryClaim,
    cacheable::{
        AccessTokenResult, ApiError as CacheableApiError, Tenant as CacheableTenant,
        User as CacheableUser,
    },
    jwt::decode_token,
};

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

        let redis_cache = redis::Cache::new(redis, prefix, ttl);

        Box::new(redis_cache.with_overlay(local_cache).with_hashing())
    }

    /// Insert an access token into the cache with a specific expiry date
    /// Cache will reject a token, which has no expiry date or its ttl is too short
    /// Cache stores either a valid token metadata or an error
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

    /// A hacky helper function that was introduced in the past
    /// specifically for the patch_me endpoint
    /// to update the access token cache with new user data
    pub async fn upsert_access_token_patch_me(
        &self,
        user: User,
        tenant: Tenant,
        access_token: &str,
    ) -> Result<(), CaptureApiError> {
        let claim = decode_token::<OnlyExpiryClaim>(access_token)
            .whatever_context::<&str, Whatever>(
                "failed to decode access token for user profile update",
            )?;

        let token_ttl = claim.exp - Utc::now();
        if token_ttl > chrono::Duration::seconds(MIN_TOKEN_TTL_SECS) {
            match token_ttl.to_std() {
                Ok(token_ttl_std) => {
                    self.access_tokens
                        .insert_with_ttl(
                            access_token.to_string(),
                            Ok((tenant.into(), user.into())),
                            token_ttl_std,
                        )
                        .await?;
                }
                Err(e) => {
                    log::debug!(
                        "abort user profile cache update due to invalid token TTL, {}",
                        Report::from_error(e)
                    );
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Caches")
    }
}
