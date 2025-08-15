// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{tenants::TenantId, time::Timestamp};

use super::OidcTenantId;

/// Information about a tenant stored in the inventory.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    bincode::Encode,
    bincode::Decode,
)]
pub struct Tenant {
    /// The id of the tenant.
    pub id: TenantId,

    /// The creation timestamp.
    #[bincode(with_serde)]
    pub created_at: Timestamp,

    /// THe updated timestamp.
    #[bincode(with_serde)]
    pub updated_at: Timestamp,

    /// The OIDC tenant id.
    pub oidc_tenant_id: OidcTenantId,
}

impl From<opentalk_db_storage::tenants::Tenant> for Tenant {
    fn from(
        opentalk_db_storage::tenants::Tenant {
            id,
            created_at,
            updated_at,
            oidc_tenant_id,
        }: opentalk_db_storage::tenants::Tenant,
    ) -> Self {
        Self {
            id,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            oidc_tenant_id: oidc_tenant_id.into(),
        }
    }
}
