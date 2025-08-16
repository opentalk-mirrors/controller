// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    tariffs::{TariffId, TariffStatus},
    tenants::TenantId,
    time::TimeZone,
    users::{DisplayName, Language, UserTitle},
};

/// The representation of a new user that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    /// The OIDC sub field of the user.
    pub oidc_sub: String,

    /// The E-Mail address of the user.
    pub email: String,

    /// The title of the user.
    pub title: UserTitle,

    /// The first name of the user.
    pub firstname: String,

    /// The last name of the user.
    pub lastname: String,

    /// The language of the user.
    pub language: Language,

    /// The display name of the user.
    pub display_name: DisplayName,

    /// The phone number of the user.
    pub phone: Option<String>,

    /// The id of the tenant to which the user belongs.
    pub tenant_id: TenantId,

    /// The id of the tariff that is assigned to the user.
    pub tariff_id: TariffId,

    /// The status of the tariff that is assigned to the user.
    pub tariff_status: TariffStatus,

    /// The URL to the avatar of the user.
    pub avatar_url: Option<String>,

    /// The time zone of the user.
    pub timezone: Option<TimeZone>,
}

impl From<opentalk_db_storage::users::NewUser> for NewUser {
    fn from(
        opentalk_db_storage::users::NewUser {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }: opentalk_db_storage::users::NewUser,
    ) -> Self {
        Self {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }
    }
}

impl From<NewUser> for opentalk_db_storage::users::NewUser {
    fn from(
        NewUser {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }: NewUser,
    ) -> Self {
        Self {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        }
    }
}
