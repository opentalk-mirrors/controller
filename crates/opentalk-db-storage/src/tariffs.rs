// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::fmt::Debug;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    features::ModuleFeatureId,
    modules::ModuleId,
    tariffs::{QuotaType, TariffId},
    users::UserId,
};
use redis_args::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::{
    schema::{external_tariffs, tariffs, users},
    utils::Jsonb,
};

#[derive(
    AsRef,
    Display,
    From,
    FromStr,
    Into,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    AsExpression,
    FromSqlRow,
    DieselNewtype,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub struct ExternalTariffId(String);

impl From<ExternalTariffId> for inventory::ExternalTariffId {
    fn from(ExternalTariffId(value): ExternalTariffId) -> Self {
        Self::from(value)
    }
}

impl From<inventory::ExternalTariffId> for ExternalTariffId {
    fn from(value: inventory::ExternalTariffId) -> Self {
        Self(value.into())
    }
}

#[derive(
    Debug,
    Clone,
    Queryable,
    Identifiable,
    Serialize,
    Deserialize,
    ToRedisArgs,
    FromRedisValue,
    PartialEq,
    Eq,
)]
#[to_redis_args(serde)]
#[from_redis_value(serde)]
pub struct Tariff {
    pub id: TariffId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub quotas: Jsonb<BTreeMap<QuotaType, u64>>,
    pub disabled_modules: Vec<Option<ModuleId>>,
    pub disabled_features: Vec<Option<ModuleFeatureId>>,
}

impl From<Tariff> for inventory::Tariff {
    fn from(
        Tariff {
            id,
            name,
            created_at,
            updated_at,
            quotas,
            disabled_modules,
            disabled_features,
        }: Tariff,
    ) -> Self {
        Self {
            id,
            name,
            created_at: created_at.into(),
            updated_at: updated_at.into(),
            quotas: quotas.0,
            disabled_modules,
            disabled_features,
        }
    }
}

impl Tariff {
    pub async fn get(conn: &mut DbConnection, id: TariffId) -> Result<Self> {
        let tariff = tariffs::table
            .filter(tariffs::id.eq(id))
            .get_result(conn)
            .await?;

        Ok(tariff)
    }

    pub async fn get_all(conn: &mut DbConnection) -> Result<Vec<Self>> {
        let tariffs = tariffs::table.load(conn).await?;

        Ok(tariffs)
    }

    pub async fn get_by_name(conn: &mut DbConnection, name: &str) -> Result<Self> {
        let query = tariffs::table.filter(tariffs::name.eq(name));

        let tariff = query.get_result(conn).await?;

        Ok(tariff)
    }

    pub async fn delete_by_id(conn: &mut DbConnection, id: TariffId) -> Result<()> {
        let query = diesel::delete(tariffs::table).filter(tariffs::id.eq(id));
        query.execute(conn).await?;
        Ok(())
    }

    pub async fn get_by_external_id(
        conn: &mut DbConnection,
        id: &ExternalTariffId,
    ) -> Result<Self> {
        let query = external_tariffs::table
            .filter(external_tariffs::external_id.eq(id))
            .inner_join(tariffs::table)
            .select(tariffs::all_columns);

        let tariff = query.get_result(conn).await?;

        Ok(tariff)
    }

    pub async fn get_by_user_id(conn: &mut DbConnection, id: &UserId) -> Result<Self> {
        let query = users::table
            .filter(users::id.eq(id))
            .inner_join(tariffs::table)
            .select(tariffs::all_columns);

        let tariff = query.get_result(conn).await?;

        Ok(tariff)
    }
}

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

#[derive(Debug, Clone, Insertable, Identifiable, Queryable)]
#[diesel(primary_key(external_id))]
pub struct ExternalTariff {
    pub external_id: ExternalTariffId,
    pub tariff_id: TariffId,
}

impl From<ExternalTariff> for inventory::ExternalTariffMapping {
    fn from(
        ExternalTariff {
            external_id,
            tariff_id,
        }: ExternalTariff,
    ) -> Self {
        Self {
            external_id: external_id.into(),
            tariff_id,
        }
    }
}

impl ExternalTariff {
    pub async fn get_all_for_tariff(
        conn: &mut DbConnection,
        tariff_id: TariffId,
    ) -> Result<Vec<ExternalTariffId>> {
        let query = external_tariffs::table
            .filter(external_tariffs::tariff_id.eq(tariff_id))
            .select(external_tariffs::external_id);
        let external_ids = query.load(conn).await?;

        Ok(external_ids)
    }

    pub async fn delete_all_for_tariff(conn: &mut DbConnection, tariff_id: TariffId) -> Result<()> {
        let query = diesel::delete(external_tariffs::table)
            .filter(external_tariffs::tariff_id.eq(tariff_id));
        query.execute(conn).await?;
        Ok(())
    }

    pub async fn delete_all_for_tariff_by_external_id(
        conn: &mut DbConnection,
        tariff_id: TariffId,
        external_ids: &[ExternalTariffId],
    ) -> Result<()> {
        let query = diesel::delete(external_tariffs::table).filter(
            external_tariffs::tariff_id
                .eq(tariff_id)
                .and(external_tariffs::external_id.eq_any(external_ids)),
        );
        query.execute(conn).await?;
        Ok(())
    }

    pub async fn insert(self, conn: &mut DbConnection) -> Result<ExternalTariff> {
        let external_tariff = self
            .insert_into(external_tariffs::table)
            .get_result(conn)
            .await?;
        Ok(external_tariff)
    }
}
