// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::assets::AssetId;

use crate::{schema::assets, tables::assets::Asset};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = assets)]
pub struct UpdateAsset {
    pub size: Option<i64>,
    pub filename: Option<String>,
}

impl UpdateAsset {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(self, conn: &mut DbConnection, asset_id: AssetId) -> Result<Asset> {
        let target = assets::table.filter(assets::id.eq(&asset_id));
        let asset = diesel::update(target).set(self).get_result(conn).await?;

        Ok(asset)
    }
}

impl From<UpdateAsset> for inventory::UpdateAsset {
    fn from(UpdateAsset { size, filename }: UpdateAsset) -> Self {
        Self { size, filename }
    }
}

impl From<inventory::UpdateAsset> for UpdateAsset {
    fn from(inventory::UpdateAsset { size, filename }: inventory::UpdateAsset) -> Self {
        Self { size, filename }
    }
}
