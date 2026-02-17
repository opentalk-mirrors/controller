// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::module_resources::{self as db};
use opentalk_inventory::{
    ModuleResource, ModuleResourceFilter, ModuleResourceInventory, ModuleResourceOperation,
    NewModuleResource,
};
use opentalk_types_common::{module_resources::ModuleResourceId, rooms::RoomId, users::UserId};
use snafu::ResultExt as _;

use crate::{
    DatabaseConnection, Result,
    error::{DatabaseSnafu, JsonOperationSnafu},
};

#[async_trait::async_trait]
impl ModuleResourceInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn create_module_resource(
        &mut self,
        resource: NewModuleResource,
    ) -> Result<ModuleResource> {
        Ok(db::NewModuleResource::from(resource)
            .insert(&mut self.inner)
            .await
            .context(DatabaseSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_module_resources(
        &mut self,
        resource_filter: ModuleResourceFilter,
    ) -> Result<Vec<ModuleResource>> {
        Ok(
            db::ModuleResource::get(&mut self.inner, resource_filter.into())
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_module_resources(
        &mut self,
    ) -> Result<Vec<(ModuleResourceId, UserId, UserId)>> {
        Ok(
            db::ModuleResource::get_all_with_creator_and_owner(&mut self.inner)
                .await
                .context(DatabaseSnafu)?,
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn patch_module_resources(
        &mut self,
        resource_filter: ModuleResourceFilter,
        operations: Vec<ModuleResourceOperation>,
    ) -> Result<Vec<ModuleResource>> {
        Ok(
            db::ModuleResource::patch(&mut self.inner, resource_filter.into(), operations)
                .await
                .context(JsonOperationSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_module_ids_for_room(
        &mut self,
        room_id: RoomId,
    ) -> Result<Vec<ModuleResourceId>> {
        Ok(
            db::ModuleResource::get_all_ids_for_room(&mut self.inner, room_id)
                .await
                .context(DatabaseSnafu)?,
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_module_resources(
        &mut self,
        resource_filter: ModuleResourceFilter,
    ) -> Result<Vec<ModuleResource>> {
        Ok(
            db::ModuleResource::delete(&mut self.inner, resource_filter.into())
                .await
                .context(DatabaseSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_all_module_resources_for_room(&mut self, room_id: RoomId) -> Result<()> {
        Ok(db::ModuleResource::delete_by_room(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?)
    }
}
