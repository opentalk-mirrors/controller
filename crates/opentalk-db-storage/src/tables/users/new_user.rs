// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::Utc;
use diesel::Insertable;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    tariffs::{TariffId, TariffStatus},
    tenants::TenantId,
    time::TimeZone,
    users::{DisplayName, UserTitle},
};

use crate::{
    newtypes::LanguageIdentifier,
    schema::users,
    tables::users::{UpdateUser, User},
};

/// Diesel insertable user struct
///
/// Represents fields that have to be provided on user insertion.
#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub oidc_sub: String,
    pub email: String,
    pub title: UserTitle,
    pub firstname: String,
    pub lastname: String,
    pub language: Option<LanguageIdentifier>,
    pub display_name: DisplayName,
    pub phone: Option<String>,
    pub tenant_id: TenantId,
    pub tariff_id: TariffId,
    pub tariff_status: TariffStatus,
    pub avatar_url: Option<String>,
    pub timezone: Option<TimeZone>,
}

impl From<inventory::NewUser> for NewUser {
    fn from(
        inventory::NewUser {
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
        }: inventory::NewUser,
    ) -> Self {
        Self {
            oidc_sub,
            email,
            title,
            firstname,
            lastname,
            language: language.map(Into::into),
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

impl NewUser {
    pub async fn insert(self, conn: &mut DbConnection) -> Result<User> {
        let query = self.insert_into(users::table);
        let user = query.get_result(conn).await?;
        Ok(user)
    }

    pub async fn insert_or_update_by_oidc_sub(
        self,
        conn: &mut DbConnection,
        enforce_display_name_on_update: bool,
    ) -> Result<User> {
        let NewUser {
            oidc_sub: _,
            email,
            title,
            firstname,
            lastname,
            language: _,
            display_name,
            phone,
            tenant_id: _,
            tariff_id,
            tariff_status,
            avatar_url,
            timezone,
        } = self.clone();
        let update_user = UpdateUser {
            title: Some(&title),
            email: Some(email),
            firstname: Some(&firstname),
            lastname: Some(&lastname),
            phone: Some(phone),
            display_name: enforce_display_name_on_update.then_some(&display_name),
            language: None,
            dashboard_theme: None,
            conference_theme: None,
            tariff_id: Some(tariff_id),
            tariff_status: Some(tariff_status),
            disabled_since: None,
            avatar_url: Some(avatar_url.as_deref()),
            timezone: Some(timezone),
            updated_at: Utc::now(),
        };
        let query = self
            .insert_into(users::table)
            .on_conflict((users::oidc_sub, users::tenant_id))
            .do_update()
            .set(update_user);
        let user = query.get_result(conn).await?;
        Ok(user)
    }
}
