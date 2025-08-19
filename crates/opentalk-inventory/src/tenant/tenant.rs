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
