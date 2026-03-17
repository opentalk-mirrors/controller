// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::fmt::Debug;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    features::ModuleFeatureId,
    modules::ModuleId,
    tariffs::{QuotaType, TariffId},
};

use crate::{schema::tariffs, tables::tariffs::Tariff, utils::Jsonb};

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = tariffs)]
pub struct UpdateTariff {
    pub name: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub quotas: Option<Jsonb<BTreeMap<QuotaType, u64>>>,
    pub disabled_modules: Option<Vec<ModuleId>>,
    pub disabled_features: Option<Vec<ModuleFeatureId>>,
}

impl From<inventory::UpdateTariff> for UpdateTariff {
    fn from(
        inventory::UpdateTariff {
            name,
            updated_at,
            quotas,
            disabled_modules,
            disabled_features,
        }: inventory::UpdateTariff,
    ) -> Self {
        Self {
            name,
            updated_at: updated_at.into(),
            quotas: quotas.map(Jsonb),
            disabled_modules,
            disabled_features,
        }
    }
}

impl UpdateTariff {
    pub async fn apply(self, conn: &mut DbConnection, tariff_id: TariffId) -> Result<Tariff> {
        let query = diesel::update(tariffs::table.filter(tariffs::id.eq(tariff_id))).set(self);
        let tariff = query.get_result(conn).await?;
        Ok(tariff)
    }
}
