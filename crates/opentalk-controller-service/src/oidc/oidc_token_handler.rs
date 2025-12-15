// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use openidconnect::AccessToken;
use opentalk_controller_utils::CaptureApiError;

use super::{OpenIdConnectUserInfo, RealmRoles, VerifyError};

/// The handler for OIDC tokens
#[async_trait::async_trait(?Send)]
pub trait OidcTokenHandler: Sync + Send {
    /// Call the OIDC's userinfo endpoint to fetch the user data associated with the access token
    async fn user_info(
        &self,
        access_token: AccessToken,
    ) -> Result<OpenIdConnectUserInfo, CaptureApiError>;

    /// Check an access token
    async fn check_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Result<RealmRoles, CaptureApiError>;

    /// Verify the access token and return its expiry timestamp.
    ///
    /// Uses the introspect endpoint if available [RFC7662](https://www.rfc-editor.org/rfc/rfc7662),
    /// otherwise the token must be a JWT.
    async fn verify_access_token(
        &self,
        access_token: &AccessToken,
    ) -> Result<Option<DateTime<Utc>>, CaptureApiError>;

    /// Verifies the signature and expiration of an ID Token encoded as JWT (Json Web Token)
    ///
    /// Only used by the deprecated login endpoint
    fn verify_id_token(&self, id_token: &str) -> Result<(), VerifyError>;
}
