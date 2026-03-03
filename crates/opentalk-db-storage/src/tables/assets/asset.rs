// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, Identifiable, JoinOnDsl, QueryDsl, Queryable,
};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    assets::{AssetId, FileSize},
    modules::ModuleId,
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    tenants::TenantId,
};

use crate::{
    paginate::Paginate,
    schema::{assets, room_assets},
};

/// Diesel resource struct
#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = assets)]
pub struct Asset {
    pub id: AssetId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub namespace: Option<ModuleId>,
    pub kind: String,
    pub filename: String,
    pub tenant_id: TenantId,
    pub size: FileSize,
}

impl From<Asset> for inventory::Asset {
    fn from(
        Asset {
            id,
            created_at,
            updated_at,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }: Asset,
    ) -> Self {
        Self {
            id,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }
    }
}

impl Asset {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, room_id: RoomId, asset_id: AssetId) -> Result<Self> {
        //FIXME: The inner_join below (as well as the room_id parameter) can be removed when assets have their own
        // permission check and don't rely on room permissions

        let query = assets::table
            .inner_join(
                room_assets::table.on(room_assets::asset_id
                    .eq(assets::id)
                    .and(room_assets::room_id.eq(room_id))),
            )
            .filter(assets::id.eq(asset_id))
            .select(assets::all_columns);

        let resource: Asset = query.get_result(conn).await?;

        Ok(resource)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_ids_for_room(
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Vec<AssetId>> {
        let query = room_assets::table
            .select(room_assets::asset_id)
            .filter(room_assets::room_id.eq(room_id));

        let assets = query.load(conn).await?;

        Ok(assets)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn count_all(conn: &mut DbConnection) -> Result<i64> {
        Ok(assets::table.count().get_result(conn).await?)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_room_paginated(
        conn: &mut DbConnection,
        room_id: RoomId,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<Self>, ItemCount)> {
        let query = assets::table
            .inner_join(room_assets::table.on(room_assets::asset_id.eq(assets::id)))
            .filter(room_assets::room_id.eq(room_id))
            .select(assets::all_columns)
            .paginate_by(limit, page);

        let resources_with_total = query.load_and_count(conn).await?;

        Ok(resources_with_total)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_rooms_paginated(
        conn: &mut DbConnection,
        room_ids: &[RoomId],
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<Self>, ItemCount)> {
        let query = assets::table
            .inner_join(room_assets::table.on(room_assets::asset_id.eq(assets::id)))
            .filter(room_assets::room_id.eq_any(room_ids))
            .select(assets::all_columns)
            .paginate_by(limit, page);

        let resources_with_total = query.load_and_count(conn).await?;

        Ok(resources_with_total)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_ids_and_size(conn: &mut DbConnection) -> Result<Vec<(AssetId, i64)>> {
        let query = assets::table
            .select((assets::id, assets::size))
            .order_by(assets::created_at.asc());

        let resources_with_total = query.load(conn).await?;

        Ok(resources_with_total)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(
        conn: &mut DbConnection,
        room_id: RoomId,
        asset_id: AssetId,
    ) -> Result<()> {
        conn.transaction(|conn| {
            async move {
                //FIXME: This check (as well as the room_id parameter) can be removed when assets have their own permission
                // check and don't rely on room permissions
                //
                // check if the asset exists for the specified room
                room_assets::table
                    .filter(
                        room_assets::asset_id
                            .eq(asset_id)
                            .and(room_assets::room_id.eq(room_id)),
                    )
                    .execute(conn)
                    .await?;

                diesel::delete(assets::table.filter(assets::id.eq(asset_id)))
                    .execute(conn)
                    .await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    /// Used for the internal deletion of assets
    ///
    /// When the request originates from a client, the [`Self::delete_by_id`] method should be used.
    #[tracing::instrument(err, skip_all)]
    pub async fn internal_delete_by_id(conn: &mut DbConnection, asset_id: &AssetId) -> Result<()> {
        let query = diesel::delete(assets::table.filter(assets::id.eq(asset_id)));

        query.execute(conn).await?;

        Ok(())
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_ids(conn: &mut DbConnection, asset_ids: &[AssetId]) -> Result<()> {
        let query = diesel::delete(assets::table.filter(assets::id.eq_any(asset_ids)));

        query.execute(conn).await?;

        Ok(())
    }
}
