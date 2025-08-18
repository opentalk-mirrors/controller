// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::{BTreeMap, BTreeSet};

use opentalk_db_storage::utils::Jsonb;
use opentalk_types_common::{features::ModuleFeatureId, modules::ModuleId, tariffs::QuotaType};

/// The representation of a new tariff that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTariff {
    /// The name of the tariff.
    pub name: String,

    /// The quotas for the tariff.
    pub quotas: BTreeMap<QuotaType, u64>,

    /// The set of disabled modules in the tariff.
    pub disabled_modules: BTreeSet<ModuleId>,

    /// The set of disabled features in the tariff.
    pub disabled_features: BTreeSet<ModuleFeatureId>,
}

impl From<opentalk_db_storage::tariffs::NewTariff> for NewTariff {
    fn from(
        opentalk_db_storage::tariffs::NewTariff {
            name,
            quotas,
            disabled_modules,
            disabled_features,
        }: opentalk_db_storage::tariffs::NewTariff,
    ) -> Self {
        Self {
            name,
            quotas: quotas.0,
            disabled_modules: BTreeSet::from_iter(disabled_modules),
            disabled_features: BTreeSet::from_iter(disabled_features),
        }
    }
}

impl From<NewTariff> for opentalk_db_storage::tariffs::NewTariff {
    fn from(
        NewTariff {
            name,
            quotas,
            disabled_modules,
            disabled_features,
        }: NewTariff,
    ) -> Self {
        Self {
            name,
            quotas: Jsonb(quotas),
            disabled_modules: Vec::from_iter(disabled_modules),
            disabled_features: Vec::from_iter(disabled_features),
        }
    }
}
