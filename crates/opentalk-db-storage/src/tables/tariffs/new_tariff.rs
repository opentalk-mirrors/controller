// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::fmt::Debug;
use std::collections::BTreeMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{features::ModuleFeatureId, modules::ModuleId, tariffs::QuotaType};

use crate::{schema::tariffs, tables::tariffs::Tariff, utils::Jsonb};

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tariffs)]
pub struct NewTariff {
    pub name: String,
    pub quotas: Jsonb<BTreeMap<QuotaType, u64>>,
    pub disabled_modules: Vec<ModuleId>,
    pub disabled_features: Vec<ModuleFeatureId>,
}

impl NewTariff {
    pub async fn insert(self, conn: &mut DbConnection) -> Result<Tariff> {
        let query = self.insert_into(tariffs::table);
        let tariff = query.get_result(conn).await?;

        Ok(tariff)
    }
}

impl From<inventory::NewTariff> for NewTariff {
    fn from(
        inventory::NewTariff {
            name,
            quotas,
            disabled_modules,
            disabled_features,
        }: inventory::NewTariff,
    ) -> Self {
        Self {
            name,
            quotas: Jsonb(quotas),
            disabled_modules: Vec::from_iter(disabled_modules),
            disabled_features: Vec::from_iter(disabled_features),
        }
    }
}
