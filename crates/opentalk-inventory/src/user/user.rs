// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    tariffs::{TariffId, TariffStatus},
    tenants::TenantId,
    time::{TimeZone, Timestamp},
    users::{DisplayName, Language, Theme, UserId, UserTitle},
};

use crate::Event;

/// The representation of a user in the inventory.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
pub struct User {
    /// The id of the user.
    pub id: UserId,

    /// The serial id of the user.
    pub id_serial: i64,

    /// The OIDC sub of the user.
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

    /// The dashboard theme.
    pub dashboard_theme: Theme,

    /// The conference theme.
    pub conference_theme: Theme,

    /// The phone number of the user.
    pub phone: Option<String>,

    /// The id of the tenant to which the user belongs.
    pub tenant_id: TenantId,

    /// The id of the tariff that is assigned to the user.
    pub tariff_id: TariffId,

    /// The status of the tariff that is assigned to the user.
    pub tariff_status: TariffStatus,

    /// Optional disabled-since timestamp.
    #[bincode(with_serde)]
    pub disabled_since: Option<Timestamp>,

    /// The URL to the avatar of the user.
    pub avatar_url: Option<String>,

    /// The time zone of the user.
    pub timezone: Option<TimeZone>,

    /// The creation timestamp.
    #[bincode(with_serde)]
    pub created_at: Timestamp,

    /// The updated timestamp.
    #[bincode(with_serde)]
    pub updated_at: Timestamp,
}

impl User {
    /// Tell whether a user is allowed to edit an event.
    pub fn can_edit(&self, event: &Event) -> bool {
        self.id == event.created_by
    }
}

impl From<opentalk_db_storage::users::User> for User {
    fn from(
        opentalk_db_storage::users::User {
            id,
            id_serial,
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            created_at,
            updated_at,
        }: opentalk_db_storage::users::User,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(Into::into),
            avatar_url,
            timezone,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }
}

impl From<User> for opentalk_db_storage::users::User {
    fn from(
        User {
            id,
            id_serial,
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            created_at,
            updated_at,
        }: User,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language,
            display_name,
            dashboard_theme,
            conference_theme,
            phone,
            tenant_id,
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(Into::into),
            avatar_url,
            timezone,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        }
    }
}
