// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    assets::AssetId, modules::ModuleId, tenants::TenantId, time::Timestamp,
};

/// Information about an asset stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// The id of the asset.
    pub id: AssetId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The updated timestamp.
    pub updated_at: Timestamp,

    /// An optional module namespace.
    pub namespace: Option<ModuleId>,

    /// The asset kind.
    pub kind: String,

    /// The filename of the asset.
    pub filename: String,

    /// The id of the tenant to which the asset belongs.
    pub tenant_id: TenantId,

    /// The size of the asset, in bytes.
    pub size: i64,
}

impl From<opentalk_db_storage::assets::Asset> for Asset {
    fn from(
        opentalk_db_storage::assets::Asset {
            id,
            created_at,
            updated_at,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }: opentalk_db_storage::assets::Asset,
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
