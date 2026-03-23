// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::tenants::TenantId;

use crate::{
    schema::tenants,
    tables::tenants::{OidcTenantId, Tenant},
};

#[derive(AsChangeset)]
#[diesel(table_name = tenants)]
pub struct UpdateTenant<'a> {
    pub updated_at: DateTime<Utc>,
    pub oidc_tenant_id: &'a OidcTenantId,
}

impl UpdateTenant<'_> {
    pub async fn apply(self, conn: &mut DbConnection, tenant_id: TenantId) -> Result<Tenant> {
        let query = diesel::update(tenants::table.filter(tenants::id.eq(tenant_id))).set(self);
        let tenant: Tenant = query.get_result(conn).await?;
        Ok(tenant)
    }
}
