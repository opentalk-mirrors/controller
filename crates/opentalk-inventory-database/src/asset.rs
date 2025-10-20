// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_database::DatabaseError;
use opentalk_db_storage::assets as db;
use opentalk_inventory::{Asset, AssetInventory, NewAsset, UpdateAsset};
use opentalk_types_common::{
    assets::{AssetId, AssetSorting},
    events::EventId,
    order::Ordering,
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    users::UserId,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result, error::DatabaseSnafu};

#[async_trait::async_trait]
impl AssetInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn create_asset_for_room(&mut self, room_id: RoomId, asset: NewAsset) -> Result<Asset> {
        Ok(db::NewAsset::from(asset)
            .insert_for_room(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_asset_from_room(&mut self, room_id: RoomId, asset_id: AssetId) -> Result<()> {
        Ok(db::Asset::delete_by_id(&mut self.inner, room_id, asset_id)
            .await
            .context(DatabaseSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_asset_for_room(&mut self, room_id: RoomId, asset_id: AssetId) -> Result<Asset> {
        Ok(db::Asset::get(&mut self.inner, room_id, asset_id)
            .await
            .context(DatabaseSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_assets_with_size(&mut self) -> Result<Vec<(AssetId, i64)>> {
        Ok(db::Asset::get_all_ids_and_size(&mut self.inner)
            .await
            .context(DatabaseSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_assets_for_room_paginated(
        &mut self,
        room_id: RoomId,
        per_page: PageSize,
        page: Page,
    ) -> Result<(Vec<Asset>, ItemCount)> {
        let (assets, overall) =
            db::Asset::get_all_for_room_paginated(&mut self.inner, room_id, per_page, page)
                .await
                .context(DatabaseSnafu)?;
        Ok((assets.into_iter().map(Into::into).collect(), overall))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_asset_ids_for_room(&mut self, room_id: RoomId) -> Result<Vec<AssetId>> {
        Ok(db::Asset::get_all_ids_for_room(&mut self.inner, room_id)
            .await
            .context(DatabaseSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_assets_for_room_owner_paginated_ordered(
        &mut self,
        user_id: UserId,
        limit: PageSize,
        page: Page,
        sort: AssetSorting,
        order: Ordering,
    ) -> Result<(Vec<(Asset, RoomId, Option<EventId>)>, ItemCount)> {
        let (items, overall) = db::get_all_for_room_owner_paginated_ordered(
            &mut self.inner,
            user_id,
            limit,
            page,
            sort,
            order,
        )
        .await
        .context(DatabaseSnafu)?;
        Ok((
            items
                .into_iter()
                .map(|(asset, room_id, event_id)| (asset.into(), room_id, event_id))
                .collect(),
            overall,
        ))
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_asset(
        &mut self,
        asset_id: AssetId,
        asset: UpdateAsset,
    ) -> Result<Option<Asset>> {
        match db::UpdateAsset::from(asset)
            .apply(&mut self.inner, asset_id)
            .await
        {
            Ok(asset) => Ok(Some(asset.into())),
            Err(DatabaseError::NotFound) => Ok(None),
            Err(e) => Err(e).context(DatabaseSnafu)?,
        }
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_asset_by_id_internal(&mut self, asset_id: AssetId) -> Result<()> {
        Ok(db::Asset::internal_delete_by_id(&mut self.inner, &asset_id)
            .await
            .context(DatabaseSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_assets_by_ids(&mut self, asset_ids: &[AssetId]) -> Result<()> {
        Ok(db::Asset::delete_by_ids(&mut self.inner, asset_ids)
            .await
            .context(DatabaseSnafu)?)
    }
}
