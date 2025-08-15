// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{assets::AssetId, modules::ModuleId, tenants::TenantId};

/// Representation of an asset that should be created in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAsset {
    /// The id of the asset.
    pub id: AssetId,

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

impl From<opentalk_db_storage::assets::NewAsset> for NewAsset {
    fn from(
        opentalk_db_storage::assets::NewAsset {
            id,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }: opentalk_db_storage::assets::NewAsset,
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

impl From<NewAsset> for opentalk_db_storage::assets::NewAsset {
    fn from(
        NewAsset {
            id,
            namespace,
            kind,
            filename,
            tenant_id,
            size,
        }: NewAsset,
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
