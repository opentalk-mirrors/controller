// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    module_resources::ModuleResourceId, rooms::RoomId, tenants::TenantId, time::Timestamp,
    users::UserId,
};

/// The representation of a module resource in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleResource {
    /// The id of the module resource.
    pub id: ModuleResourceId,

    /// The tenant to which this module resource is associated.
    pub tenant_id: TenantId,

    /// The id of the room to which the module resource belongs..
    pub room_id: RoomId,

    /// The id of the user who created the module resource.
    pub created_by: UserId,

    /// The creation timestamp.
    pub created_at: Timestamp,

    /// The id of the user who last updated the module resource.
    pub updated_at: Timestamp,

    /// The namespace of the module resource.
    pub namespace: String,

    /// An optional tag for the module resource, may be used by the corresponding module.
    pub tag: Option<String>,

    /// The module resource data.
    pub data: serde_json::Value,
}

impl From<opentalk_db_storage::module_resources::ModuleResource> for ModuleResource {
    fn from(
        opentalk_db_storage::module_resources::ModuleResource {
            id,
            tenant_id,
            room_id,
            created_by,
            created_at,
            updated_at,
            namespace,
            tag,
            data,
        }: opentalk_db_storage::module_resources::ModuleResource,
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

impl From<ModuleResource> for opentalk_db_storage::module_resources::ModuleResource {
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
