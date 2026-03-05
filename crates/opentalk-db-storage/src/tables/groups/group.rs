// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    tenants::TenantId,
    users::{GroupId, GroupName, UserId},
};

use crate::{
    schema::{groups, user_groups},
    tables::groups::SerialGroupId,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Queryable, Insertable, Identifiable)]
#[diesel(table_name = groups)]
pub struct Group {
    pub id: GroupId,
    pub id_serial: SerialGroupId,
    pub name: GroupName,
    pub tenant_id: TenantId,
}

impl From<Group> for inventory::Group {
    fn from(
        Group {
            id,
            id_serial: _,
            name,
            tenant_id,
        }: Group,
    ) -> Self {
        Self {
            id,
            name,
            tenant_id,
        }
    }
}

impl Group {
    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_user(conn: &mut DbConnection, user_id: UserId) -> Result<Vec<Group>> {
        let query = user_groups::table
            .inner_join(groups::table)
            .filter(user_groups::user_id.eq(user_id))
            .select(groups::all_columns)
            .order_by(groups::id_serial);

        let groups: Vec<Group> = query.load(conn).await?;

        Ok(groups)
    }
}
