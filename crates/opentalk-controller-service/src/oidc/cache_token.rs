// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2
use chrono::Utc;
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{Tenant, User};
use snafu::{Report, ResultExt, Whatever};

use super::{Cache, OnlyExpiryClaim, jwt::decode_token};

/// A hacky helper function that was introduced in the past
/// specifically for the patch_me endpoint
/// to update the access token cache with new user data
///
/// TODO: Clarify, if this function can be merged with a general upsert or insert
///
///
/// TODO: For some reason, we neither check, if the token cached neither verify it via introspection
///       neither here not in the caller function.
///       This might be a security issue.
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
    if token_ttl > chrono::Duration::seconds(10) {
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
