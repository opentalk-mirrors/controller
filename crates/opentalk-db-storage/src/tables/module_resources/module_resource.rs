// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Identifiable, QueryDsl, Queryable, prelude::*, sql_types::Jsonb};
use diesel_async::RunQueryDsl;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    module_resources::ModuleResourceId, rooms::RoomId, tenants::TenantId, users::UserId,
};

use crate::{
    module_resources::{Filter, JsonOperationError},
    schema::{module_resources, rooms},
};

#[derive(Debug, Clone, Eq, PartialEq, Queryable, Identifiable, Insertable)]
#[diesel(table_name = module_resources)]
pub struct ModuleResource {
    pub id: ModuleResourceId,
    pub tenant_id: TenantId,
    pub room_id: RoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub namespace: String,
    pub tag: Option<String>,
    pub data: serde_json::Value,
}

impl From<ModuleResource> for inventory::ModuleResource {
    fn from(
        ModuleResource {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at,
            updated_at,
            namespace,
            tag,
            data,
        }: ModuleResource,
    ) -> Self {
        Self {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            namespace,
            tag,
            data,
        }
    }
}

impl From<inventory::ModuleResource> for ModuleResource {
    fn from(
        inventory::ModuleResource {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at,
            updated_at,
            namespace,
            tag,
            data,
        }: inventory::ModuleResource,
    ) -> Self {
        Self {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            namespace,
            tag,
            data,
        }
    }
}

impl ModuleResource {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, filter: Filter) -> Result<Vec<Self>> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| DatabaseError::Custom {
                message: "Missing filter for get module_resource query".to_owned(),
            })?;

        let query = module_resources::table.filter(filter);

        let module_resources = query.get_results(conn).await?;

        Ok(module_resources)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_ids_for_room(
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Vec<ModuleResourceId>> {
        let query = module_resources::table
            .select(module_resources::id)
            .filter(module_resources::room_id.eq(room_id));

        let module_resource_ids = query.load(conn).await?;

        Ok(module_resource_ids)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_with_creator_and_owner(
        conn: &mut DbConnection,
    ) -> Result<Vec<(ModuleResourceId, UserId, UserId)>> {
        let query = module_resources::table.inner_join(rooms::table).select((
            module_resources::id,
            module_resources::created_by,
            rooms::created_by,
        ));

        let module_resources_with_creator = query.load(conn).await?;

        Ok(module_resources_with_creator)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete(conn: &mut DbConnection, filter: Filter) -> Result<Vec<ModuleResource>> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| DatabaseError::Custom {
                message: "Missing filter for delete query".to_owned(),
            })?;

        let query = diesel::delete(module_resources::table).filter(filter);

        Ok(query.get_results(conn).await?)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        diesel::delete(module_resources::table)
            .filter(module_resources::room_id.eq(room_id))
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Update the targeted json data by a set of json patch
    /// [ModuleResourceOperations](inventory::ModuleResourceOperation).
    #[tracing::instrument(err, skip_all)]
    pub async fn patch(
        conn: &mut DbConnection,
        filter: Filter,
        operations: Vec<inventory::ModuleResourceOperation>,
    ) -> Result<Vec<ModuleResource>, JsonOperationError> {
        let filter = filter
            .into_diesel_filter()
            .ok_or_else(|| JsonOperationError::Database {
                source: DatabaseError::Custom {
                    message: "Missing filter for patch query".to_owned(),
                },
            })?;

        let query = diesel::update(module_resources::table).filter(filter);

        let query = query.set(module_resources::data.eq(ot_patch_json(
            module_resources::data,
            serde_json::to_value(operations).unwrap(),
        )));

        let module_resources = query.get_results(conn).await?;

        Ok(module_resources)
    }
}

define_sql_function! {
    fn ot_patch_json(target: Jsonb, changeset: Jsonb) -> Jsonb;
}
