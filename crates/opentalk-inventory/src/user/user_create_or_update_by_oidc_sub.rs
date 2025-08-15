// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    tariffs::{TariffId, TariffStatus},
    tenants::TenantId,
    time::TimeZone,
    users::{DisplayName, Language, UserTitle},
};

/// Information about a user identified by the OIDC `sub` field.
///
/// The user with that OIDC `sub` should either be created, or updated if that
/// OIDC `sub` already exists in the inventory.
#[derive(Debug, PartialEq, Eq)]
pub struct UserCreateOrUpdateByOidcSub {
    /// The OIDC `sub` field value
    pub oidc_sub: String,

    /// The E-Mail address of the user
    pub email: String,

    /// The title of the user
    pub title: UserTitle,

    /// The first name of the user
    pub firstname: String,

    /// The last name of the user
    pub lastname: String,

    /// The display name of the user
    pub display_name: DisplayName,

    /// An optional phone number of the user
    pub phone: Option<String>,

    /// The id of the tenant to which the user belongs
    pub tenant_id: TenantId,

    /// The id of the tariff assigned to the user
    pub tariff_id: TariffId,

    /// The status of the tariff assignment
    pub tariff_status: TariffStatus,

    /// An optional url to the avatar of the user
    pub avatar_url: Option<String>,

    /// An optional timezone for the user
    pub timezone: Option<TimeZone>,

    /// The language of the user
    pub language: Language,
}
