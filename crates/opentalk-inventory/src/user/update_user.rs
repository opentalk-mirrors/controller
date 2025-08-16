// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    tariffs::{TariffId, TariffStatus},
    time::{TimeZone, Timestamp},
    users::{DisplayName, Language, Theme, UserTitle},
};

/// Representation of an update to a user in the inventory.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct UpdateUser<'a> {
    /// Update the title.
    pub title: Option<&'a UserTitle>,

    /// Update the e-mail address.
    pub email: Option<String>,

    /// Update the first name.
    pub firstname: Option<&'a str>,

    /// Update the last name.
    pub lastname: Option<&'a str>,

    /// Update the phone number.
    pub phone: Option<Option<String>>,

    /// Update the display name.
    pub display_name: Option<&'a DisplayName>,

    /// Update the language.
    pub language: Option<&'a Language>,

    /// Update the dashboard theme.
    pub dashboard_theme: Option<&'a Theme>,

    /// Update the conference theme.
    pub conference_theme: Option<&'a Theme>,

    // The tenant_id should never be updated!
    //pub tenant_id: Option<TenantId>,
    /// Update the tariff id.
    pub tariff_id: Option<TariffId>,

    /// Update the tariff status.
    pub tariff_status: Option<TariffStatus>,

    /// Update the optional disabled-since timestamp.
    pub disabled_since: Option<Option<Timestamp>>,

    /// Update the avatar url.
    pub avatar_url: Option<Option<&'a str>>,

    /// Update the timezone.
    pub timezone: Option<Option<TimeZone>>,

    /// Update the updated_at timestamp.
    pub updated_at: Timestamp,
}

impl<'a> From<UpdateUser<'a>> for opentalk_db_storage::users::UpdateUser<'a> {
    fn from(
        UpdateUser {
            title,
            email,
            firstname,
            lastname,
            phone,
            display_name,
            language,
            dashboard_theme,
            conference_theme,
            tariff_id,
            tariff_status,
            disabled_since,
            avatar_url,
            timezone,
            updated_at,
        }: UpdateUser<'a>,
    ) -> opentalk_db_storage::users::UpdateUser<'a> {
        opentalk_db_storage::users::UpdateUser {
            title,
            email,
            firstname,
            lastname,
            phone,
            display_name,
            language,
            dashboard_theme,
            conference_theme,
            tariff_id,
            tariff_status,
            disabled_since: disabled_since.map(|d| d.map(Into::into)),
            avatar_url,
            timezone,
            updated_at: updated_at.into(),
        }
    }
}
