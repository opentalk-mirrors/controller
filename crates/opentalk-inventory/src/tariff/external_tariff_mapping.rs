// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::tariffs::TariffId;

use super::ExternalTariffId;

/// The representation of an external tariff reference in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalTariffMapping {
    /// The external id of the tariff, supplied e.g. by the OIDC provider.
    pub external_id: ExternalTariffId,

    /// The id of the tariff inside the service to which the external tariff id is mapped..
    pub tariff_id: TariffId,
}

impl From<opentalk_db_storage::tariffs::ExternalTariff> for ExternalTariffMapping {
    fn from(
        opentalk_db_storage::tariffs::ExternalTariff {
            external_id,
            tariff_id,
        }: opentalk_db_storage::tariffs::ExternalTariff,
    ) -> Self {
        Self {
            external_id: external_id.into(),
            tariff_id,
        }
    }
}
