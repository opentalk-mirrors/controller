// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::groups::get_or_create_groups_by_name;
use opentalk_inventory::{Group, GroupInventory, error::StorageBackendSnafu};
use opentalk_types_common::{
    tenants::TenantId,
    users::{GroupName, UserId},
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl GroupInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn get_or_create_groups_by_name(
        &mut self,
        groups: &[(TenantId, GroupName)],
    ) -> Result<Vec<Group>> {
        Ok(get_or_create_groups_by_name(&mut self.inner, groups)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_groups_for_user(&mut self, user_id: UserId) -> Result<Vec<Group>> {
        Ok(
            opentalk_db_storage::groups::Group::get_all_for_user(&mut self.inner, user_id)
                .await
                .context(StorageBackendSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }
}
