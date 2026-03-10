// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::{ExpressionMethods, Identifiable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;

use crate::{
    schema::jobs,
    tables::jobs::{JobType, SerialJobId},
};

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
#[diesel(table_name = jobs)]
pub struct Job {
    pub id: SerialJobId,
    pub name: String,
    pub kind: JobType,
    pub parameters: serde_json::Value,
    pub timeout_secs: i32,
    pub recurrence: String,
}

impl From<Job> for inventory::Job {
    fn from(
        Job {
            id,
            name,
            kind,
            parameters,
            timeout_secs,
            recurrence,
        }: Job,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            kind: kind.into(),
            parameters,
            timeout_secs,
            recurrence,
        }
    }
}

impl From<inventory::Job> for Job {
    fn from(
        inventory::Job {
            id,
            name,
            kind,
            parameters,
            timeout_secs,
            recurrence,
        }: inventory::Job,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            kind: kind.into(),
            parameters,
            timeout_secs,
            recurrence,
        }
    }
}

impl Job {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, id: SerialJobId) -> Result<Self> {
        let query = jobs::table.filter(jobs::id.eq(id));

        let job: Job = query.get_result(conn).await?;

        Ok(job)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all(conn: &mut DbConnection) -> Result<Vec<Self>> {
        let query = jobs::table;
        let job = query.load(conn).await?;
        Ok(job)
    }
}
