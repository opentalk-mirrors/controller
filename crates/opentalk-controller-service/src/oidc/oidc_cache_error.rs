// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::TimeDelta;
use opentalk_cache::CacheError;
use snafu::Snafu;

use super::cacheable::DecodeFromCacheError;

/// OIDC cache errors
#[derive(Debug, Snafu)]
pub enum OidcCacheError {
    /// Underlying cache storage errors
    #[snafu(display("cache error: {source}"))]
    Cache {
        /// The source error from the cache storage
        source: CacheError,
    },

    /// Errors when decoding cached values
    #[snafu(display("cache error while decoding: {source}"))]
    DecodeFromCacheError {
        /// The source error of the decoding operation
        source: DecodeFromCacheError,
    },

    /// Error on attempt to cache a token, which expires too soon
    #[snafu(display("token expires soon and will not be cached (ttl={ttl:?})"))]
    TokenTtlTooShort {
        /// Provided token TTL, which was considered too short
        ttl: TimeDelta,
    },

    /// Error on attempt to cache a token, which has no expiry date
    #[snafu(display("token has no expiry and will not be cached"))]
    NoExpiryForToken,

    /// Error on attempt to update a token, which do not exist in the cache yet
    #[snafu(display("token doen't exist in cache and cannot be updated"))]
    CannotUpdateNonExistingToken,

    /// Error indicating that a formerly valid access token has been revoked by sub logout
    #[snafu(display("token has been revoked by sub logout"))]
    RevokedByLogout,
}

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
