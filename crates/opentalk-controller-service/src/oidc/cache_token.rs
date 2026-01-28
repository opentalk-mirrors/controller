// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2
use chrono::{DateTime, Utc};
use openidconnect::AccessToken;
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Tenant, User};
use snafu::{Report, ResultExt, Whatever};

use super::{Cache, OnlyExpiryClaim, cache::Result, jwt::decode_token};
use crate::{
    caching::cacheable::{
        ApiError as CacheableApiError, Tenant as CacheableTenant, User as CacheableUser,
    },
    oidc::cache::AccesTokenCacheError,
};

const MIN_TOKEN_TTL_SECS: i64 = 10;

/// Insert an access token into the cache with a specific expiry date
/// Cache will reject a token, which has no expiry date or its ttl is too short
/// Cache stores either a valid token metadata or an error
pub async fn insert_access_token(
    cache: &Cache,
    access_token: &AccessToken,
    value: Result<(Tenant, User), CaptureApiError>,
    maybe_expires_at: Option<DateTime<Utc>>,
) -> Result<()> {
    // if we have a expiry date that is more then 10 seconds in the future, cache the response.
    let Some(expires_at) = maybe_expires_at else {
        return Err(AccesTokenCacheError::NoExpiryForToken);
    };

    let token_ttl = expires_at - Utc::now();

    if token_ttl <= chrono::Duration::seconds(MIN_TOKEN_TTL_SECS) {
        return Err(AccesTokenCacheError::TokenTtlTooShort { ttl: token_ttl });
    }

    let value = match value {
        Ok((tenant, user)) => Ok((CacheableTenant::from(tenant), CacheableUser::from(user))),
        Err(e) => Err(CacheableApiError::from(e)),
    };

    cache
        .access_tokens
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
    cache: &Cache,
    user: User,
    tenant: Tenant,
    access_token: &str,
) -> Result<(), CaptureApiError> {
    let claim = decode_token::<OnlyExpiryClaim>(access_token).whatever_context::<&str, Whatever>(
        "failed to decode access token for user profile update",
    )?;

    let token_ttl = claim.exp - Utc::now();
    if token_ttl > chrono::Duration::seconds(MIN_TOKEN_TTL_SECS) {
        match token_ttl.to_std() {
            Ok(token_ttl_std) => {
                cache
                    .access_tokens
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
