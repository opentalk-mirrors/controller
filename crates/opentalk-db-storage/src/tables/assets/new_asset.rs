// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::Insertable;
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{assets::AssetId, modules::ModuleId, rooms::RoomId, tenants::TenantId};

use crate::{
    schema::{assets, room_assets},
    tables::assets::{Asset, RoomAsset},
};

#[derive(Debug, Insertable)]
#[diesel(table_name = assets)]
pub struct NewAsset {
    pub id: AssetId,
    pub namespace: Option<ModuleId>,
    pub kind: String,
    pub filename: String,
    pub tenant_id: TenantId,
    pub size: i64,
}

impl From<inventory::NewAsset> for NewAsset {
    fn from(
        inventory::NewAsset {
            id,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }: inventory::NewAsset,
    ) -> Self {
        Self {
            id,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }
    }
}

impl NewAsset {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert_for_room(self, conn: &mut DbConnection, room_id: RoomId) -> Result<Asset> {
        conn.transaction(|conn| {
            async move {
                let asset: Asset = self.insert_into(assets::table).get_result(conn).await?;

                RoomAsset {
                    room_id,
                    asset_id: asset.id,
                }
                .insert_into(room_assets::table)
                .execute(conn)
                .await?;

                Ok(asset)
            }
            .scope_boxed()
        })
        .await
    }
}
