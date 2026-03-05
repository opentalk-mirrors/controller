// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::{tenants::TenantId, users::GroupName};

use crate::{schema::groups, tables::groups::Group};

#[derive(Debug, Insertable)]
#[diesel(table_name = groups)]
pub struct NewGroup<'a> {
    pub name: &'a GroupName,
    pub tenant_id: TenantId,
}

impl NewGroup<'_> {
    /// Insert the new group. If the group already exists for the OIDC issuer the group will be
    /// returned instead
    #[tracing::instrument(err, skip_all)]
    pub async fn insert_or_get(self, conn: &mut DbConnection) -> Result<Group> {
        conn.transaction(|conn| {
            async move {
                let query = groups::table
                    .select(groups::all_columns)
                    .filter(groups::name.eq(&self.name));

                let group: Option<Group> = query.first(conn).await.optional()?;

                let group = if let Some(group) = group {
                    group
                } else {
                    diesel::insert_into(groups::table)
                        .values(self)
                        .get_result(conn)
                        .await?
                };

                Ok(group)
            }
            .scope_boxed()
        })
        .await
    }
}
