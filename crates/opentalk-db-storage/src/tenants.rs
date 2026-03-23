// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};

use crate::schema::tenants;
pub use crate::tables::tenants::{NewTenant, OidcTenantId, Tenant, UpdateTenant};

/// Get or create a tenant by name
pub async fn get_or_create_tenant_by_oidc_id(
    conn: &mut DbConnection,
    oidc_tenant_id: &OidcTenantId,
) -> Result<Tenant> {
    let present_tenant: Option<Tenant> = tenants::table
        .select(tenants::all_columns)
        .filter(tenants::oidc_tenant_id.eq(oidc_tenant_id))
        .get_result(conn)
        .await
        .optional()?;

    if let Some(tenant) = present_tenant {
        Ok(tenant)
    } else {
        let new_tenant = NewTenant { oidc_tenant_id };

        let new_tenant: Tenant = diesel::insert_into(tenants::table)
            .values(new_tenant)
            .returning(tenants::all_columns)
            .get_result(conn)
            .await?;

        Ok(new_tenant)
    }
}
