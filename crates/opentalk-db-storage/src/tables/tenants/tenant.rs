// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{tenants::TenantId, users::UserId};
use serde::{Deserialize, Serialize};

use crate::{
    schema::{tenants, users},
    tables::tenants::OidcTenantId,
};

#[derive(Debug, Clone, Queryable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = tenants)]
pub struct Tenant {
    pub id: TenantId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub oidc_tenant_id: OidcTenantId,
}

impl From<Tenant> for inventory::Tenant {
    fn from(
        Tenant {
            id,
            created_at,
            updated_at,
            oidc_tenant_id,
        }: Tenant,
    ) -> Self {
        Self {
            id,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            oidc_tenant_id: oidc_tenant_id.into(),
        }
    }
}

impl Tenant {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, id: TenantId) -> Result<Tenant> {
        let query = tenants::table.filter(tenants::id.eq(id));
        let tenant = query.get_result(conn).await?;
        Ok(tenant)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_by_oidc_id(
        conn: &mut DbConnection,
        id: OidcTenantId,
    ) -> Result<Option<Tenant>> {
        let query = tenants::table.filter(tenants::oidc_tenant_id.eq(id));
        let tenant = query.get_result(conn).await.optional()?;
        Ok(tenant)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_user(conn: &mut DbConnection, user_id: UserId) -> Result<Tenant> {
        let query = users::table
            .inner_join(tenants::table)
            .filter(users::id.eq(user_id))
            .select(tenants::all_columns);

        let tenant = query.get_result(conn).await?;

        Ok(tenant)
    }

    pub async fn get_all(conn: &mut DbConnection) -> Result<Vec<Tenant>> {
        let tenants = tenants::table.load(conn).await?;
        Ok(tenants)
    }
}
