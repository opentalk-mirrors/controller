// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{ops::Deref, str::FromStr as _};

use icu_locid::LanguageIdentifier;
use openidconnect::{
    AccessToken, ClientId, ClientSecret, LocalizedClaim, TokenIntrospectionResponse as _,
    UserInfoClaims, core::CoreGenderClaim,
};
use opentalk_controller_utils::CaptureApiError;
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::time::TimeZone;
use reqwest11::header::{HeaderMap, HeaderName, HeaderValue};
use snafu::{OptionExt as _, ResultExt as _, Whatever};
use url::Url;

use super::{
    IntrospectInfo, OnlyExpiryClaim, OpenIdConnectUserInfo, OpenTalkAdditionalClaims,
    ProviderClient, VerifyError, http, jwt,
};
use crate::Result;

/// The `OidcContext` contains all information about the Oidc provider and permissions matrix.
#[derive(Debug)]
pub struct OidcContext {
    /// The URL used by the frontend for authentication
    pub frontend_auth_base_url: Url,
    /// The provider client
    pub provider: ProviderClient,
    /// The HTTP client
    http_client: reqwest11::Client,

    /// The HTTP client for calling the introspection endpoint.
    ///
    /// In some cases, e.g. when running in a test setup, the client must send different kinds
    /// of requests when performing the OIDC introspection (mainly because Keycloak requires
    /// the introspection request go to the same host as the one that issued the frontend token).
    ///
    /// This will be set to a different client if:
    /// - The `OIDC_INTROSPECT_AND_USERINFO_SET_X_FORWARDED_HOST` environment
    ///   variable is set to `true` (case-insensitive) or `1`.
    /// - The `frontend_auth_base_url` differs from the `controller_auth_base_url`.
    ///
    /// This client adds a `x-forwarded-host` header derived from the frontend auth base url,
    /// using its host part, and if present, the port as well.
    http_introspect_and_userinfo_client: Option<reqwest11::Client>,
}

impl OidcContext {
    /// Creates the OidcContext.
    /// This reads the OIDC provider configuration and tries to fetch the metadata from it.
    /// If a provider is misconfigured or not reachable this function will fail.
    #[tracing::instrument(name = "oidc_discover", skip(client_secret))]
    pub async fn new(
        frontend_auth_base_url: Url,
        controller_auth_base_url: Url,
        client_id: ClientId,
        client_secret: ClientSecret,
    ) -> Result<Self> {
        let http_client = http::make_client(None).whatever_context("Failed to make http client")?;

        let http_introspect_and_userinfo_client =
            match std::env::var("OIDC_INTROSPECT_AND_USERINFO_SET_X_FORWARDED_HOST").ok() {
                Some(v) if v.eq_ignore_ascii_case("true") || v == "1" => {
                    let host = frontend_auth_base_url
                        .host_str()
                        .whatever_context("Frontend auth base url doesn't have a host part")?;
                    let port_appendix = frontend_auth_base_url
                        .port()
                        .map(|p| format!(":{p}"))
                        .unwrap_or_default();
                    let forwarded_host = format!("{host}{port_appendix}");
                    let mut default_headers = HeaderMap::new();
                    _ = default_headers.insert(
                        HeaderName::from_str("X-Forwarded-Host")
                            .whatever_context("Invalid header header name for introspect client")?,
                        HeaderValue::from_str(&forwarded_host)
                            .whatever_context("Invalid header value for introspect client")?,
                    );
                    Some(
                        http::make_client(Some(default_headers))
                            .whatever_context("Failed to make oidc introspection http client")?,
                    )
                }
                Some(_) | None => None,
            };

        let client = ProviderClient::discover(
            http_client.clone(),
            controller_auth_base_url,
            client_id,
            client_secret,
        )
        .await
        .whatever_context("Failed to discover provider client")?;

        Ok(Self {
            frontend_auth_base_url,
            provider: client,
            http_client,
            http_introspect_and_userinfo_client,
        })
    }

    /// Verifies the signature and expiration of an AccessToken encoded as JWT (Json Web Token)
    ///
    /// This is used if the OpenID Connect Provider does not support introspection endpoints.
    #[tracing::instrument(name = "oidc_verify_access_token", skip(self, access_token))]
    pub fn verify_jwt_token<C: jwt::VerifyClaims>(
        &self,
        access_token: &AccessToken,
    ) -> Result<C, VerifyError> {
        jwt::verify::<C>(
            self.provider.metadata.jwks(),
            access_token.secret().as_str(),
        )
    }

    /// Returns if the configured provider support introspection
    pub fn supports_introspect(&self) -> bool {
        self.provider
            .metadata
            .additional_metadata()
            .introspection_endpoint
            .is_some()
    }

    /// Call the OIDC's userinfo endpoint to fetch the user data associated with the access token
    #[tracing::instrument(name = "oidc_user_info", skip_all)]
    pub async fn introspect(&self, access_token: AccessToken) -> Result<IntrospectInfo> {
        let client = self
            .http_introspect_and_userinfo_client
            .as_ref()
            .unwrap_or(&self.http_client);
        let claims = self
            .provider
            .client
            .introspect(&access_token)
            .whatever_context("Failed to build AccessToken introspect request")?
            .request_async(http::async_http_client(client.clone()))
            .await
            .whatever_context("AccessToken introspect request failed")?;

        Ok(IntrospectInfo {
            active: claims.active(),
            exp: claims.exp(),
        })
    }

    /// Call the OIDC's userinfo endpoint to fetch the user data associated with the access token
    #[tracing::instrument(err, name = "oidc_user_info", skip_all)]
    pub async fn user_info(
        &self,
        access_token: AccessToken,
    ) -> Result<OpenIdConnectUserInfo, CaptureApiError> {
        let client = self
            .http_introspect_and_userinfo_client
            .as_ref()
            .unwrap_or(&self.http_client);
        let claims: UserInfoClaims<OpenTalkAdditionalClaims, CoreGenderClaim> = self
            .provider
            .client
            .user_info(access_token, None)
            .whatever_context::<_, Whatever>("Failed to build userinfo request")?
            .request_async(http::async_http_client(client.clone()))
            .await
            .whatever_context::<_, Whatever>("Failed to fetch userinfo")?;

        let locale_parse_result = claims.locale().map(|loc| (loc, loc.parse()));
        let locale: Option<LanguageIdentifier> = match locale_parse_result {
            // Locale exists and has correct BCP47 format
            Some((_, Ok(lang))) => Some(lang),
            // Locale exists but has wrong format
            Some((loc, Err(_))) => {
                log::warn!(
                    "Invalid locale value in token for OIDC sub {}: \"{}\"",
                    claims.subject().as_str(),
                    loc.as_str(),
                );
                None
            }
            None => None,
        };

        let timezone_parse_result = claims.zoneinfo().map(|zi| (zi, zi.parse()));
        let timezone: Option<TimeZone> = match timezone_parse_result {
            // Zoneinfo exists and has correct IANA format
            Some((_, Ok(tz))) => Some(tz),
            // Zoneinfo exists but has wrong format
            Some((zi, Err(_))) => {
                log::warn!(
                    "Invalid zoneinfo value in token for OIDC sub {}: \"{}\"",
                    claims.subject().as_str(),
                    zi.as_str(),
                );
                None
            }
            None => None,
        };

        fn expect_present_localized<'a, T>(
            t: Option<&'a LocalizedClaim<T>>,
            name: &str,
        ) -> Result<&'a T, CaptureApiError> {
            match t.and_then(|c| c.get(None)) {
                Some(t) => Ok(t),
                None => Err(ApiError::bad_request()
                    .with_message(format!(
                        "userinfo claims are missing mandatory '{name}' field"
                    ))
                    .into()),
            }
        }

        fn expect_present<'a, T>(t: Option<&'a T>, name: &str) -> Result<&'a T, CaptureApiError> {
            match t {
                Some(t) => Ok(t),
                None => Err(ApiError::bad_request()
                    .with_message(format!(
                        "userinfo claims are missing mandatory '{name}' field"
                    ))
                    .into()),
            }
        }

        fn optional<T: Deref<Target = String>>(t: Option<&LocalizedClaim<T>>) -> Option<String> {
            t.and_then(|c| c.get(None)).map(|c| c.to_string())
        }

        Ok(OpenIdConnectUserInfo {
            sub: claims.subject().to_string(),
            email: expect_present(claims.email(), "email")?.to_string(),
            firstname: expect_present_localized(claims.given_name(), "given_name")?.to_string(),
            lastname: expect_present_localized(claims.family_name(), "family_name")?.to_string(),
            avatar_url: optional(claims.picture()),
            locale,
            timezone,
            groups: claims.additional_claims().x_grp.clone(),
            phone_number: claims
                .phone_number()
                .map(|phone_number| phone_number.to_string()),
            display_name: optional(claims.nickname()),
            tenant_id: claims.additional_claims().tenant_id.clone(),
            tariff_id: claims.additional_claims().tariff_id.clone(),
            tariff_status: claims.additional_claims().tariff_status.clone(),
        })
    }

    /// Verifies the signature and expiration of an ID Token encoded as JWT (Json Web Token)
    ///
    /// Only used by the deprecated login endpoint
    #[tracing::instrument(name = "oidc_verify_id_token", skip_all)]
    pub fn verify_id_token(&self, id_token: &str) -> Result<(), VerifyError> {
        let _ = jwt::verify::<OnlyExpiryClaim>(self.provider.metadata.jwks(), id_token)?;
        Ok(())
    }

    /// Returns the provider URL
    pub fn provider_url(&self) -> String {
        self.frontend_auth_base_url.to_string()
    }
}
