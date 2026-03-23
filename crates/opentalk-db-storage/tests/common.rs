// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_database::DbConnection;
use opentalk_db_storage::{
    self as db,
    users::{NewUser, User},
};
use opentalk_types_common::{tariffs::TariffStatus, users::UserTitle};

pub async fn make_user(
    conn: &mut DbConnection,
    firstname: &str,
    lastname: &str,
    display_name: &str,
) -> User {
    let tenant = db::queries::tenants::get_or_create_tenant_by_oidc_id(
        conn,
        &db::tables::tenants::OidcTenantId::from("default".to_owned()),
    )
    .await
    .unwrap();

    let tariff = db::queries::tariffs::get_tariff_by_name(conn, "OpenTalkDefaultTariff")
        .await
        .unwrap();

    NewUser {
        email: format!(
            "{}.{}@example.org",
            firstname.to_lowercase(),
            lastname.to_lowercase()
        ),
        title: UserTitle::new(),
        firstname: firstname.into(),
        lastname: lastname.into(),
        avatar_url: None,
        display_name: display_name.parse().expect("valid display name"),
        language: Some("de".parse().expect("valid language")),
        oidc_sub: format!("{firstname}{lastname}"),
        phone: None,
        tenant_id: tenant.id,
        tariff_id: tariff.id,
        tariff_status: TariffStatus::Default,
        timezone: None,
    }
    .insert(conn)
    .await
    .unwrap()
}
