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
    users::UserId,
};
use redis_args::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::{
    schema::{external_tariffs, tariffs, users},
    tables::tariffs::ExternalTariffId,
    utils::Jsonb,
};

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
#[diesel(table_name = tariffs)]
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
