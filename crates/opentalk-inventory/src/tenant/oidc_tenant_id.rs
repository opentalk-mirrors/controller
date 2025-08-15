// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The tenant id communicated as an OIDC claim.
#[derive(
    derive_more::AsRef,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct OidcTenantId(String);

impl OidcTenantId {
    /// Extracts a string slice containing the ´OidcTenantId´.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for OidcTenantId {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

impl From<opentalk_db_storage::tenants::OidcTenantId> for OidcTenantId {
    fn from(value: opentalk_db_storage::tenants::OidcTenantId) -> Self {
        Self(String::from(value))
    }
}

impl From<OidcTenantId> for opentalk_db_storage::tenants::OidcTenantId {
    fn from(OidcTenantId(value): OidcTenantId) -> Self {
        value.into()
    }
}

impl From<&OidcTenantId> for opentalk_db_storage::tenants::OidcTenantId {
    fn from(OidcTenantId(value): &OidcTenantId) -> Self {
        value.as_str().into()
    }
}
