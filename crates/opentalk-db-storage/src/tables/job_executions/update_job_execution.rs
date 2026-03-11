// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;

use crate::{
    schema::job_executions,
    tables::job_executions::{JobExecution, JobStatus},
    tables::jobs::SerialJobId,
};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = job_executions)]
pub struct UpdateJobExecution {
    pub ended_at: Option<DateTime<Utc>>,
    pub job_status: Option<JobStatus>,
}

impl From<inventory::UpdateJobExecution> for UpdateJobExecution {
    fn from(
        inventory::UpdateJobExecution {
            ended_at,
            job_status,
        }: inventory::UpdateJobExecution,
    ) -> Self {
        Self {
            ended_at: ended_at.map(Into::into),
            job_status: job_status.map(Into::into),
        }
    }
}

impl UpdateJobExecution {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(self, conn: &mut DbConnection, id: SerialJobId) -> Result<JobExecution> {
        let target = job_executions::table.filter(job_executions::id.eq(&id));
        let job_execution = diesel::update(target).set(self).get_result(conn).await?;

        Ok(job_execution)
    }
}
