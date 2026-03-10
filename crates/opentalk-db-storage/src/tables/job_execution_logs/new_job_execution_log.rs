// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::Insertable;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;

use crate::{
    schema::job_execution_logs,
    tables::{
        job_execution_logs::{JobExecutionLog, LogLevel},
        jobs::SerialJobId,
    },
};

#[derive(Debug, Insertable)]
#[diesel(table_name = job_execution_logs)]
pub struct NewJobExecutionLog {
    pub execution_id: SerialJobId,
    pub logged_at: DateTime<Utc>,
    pub log_level: LogLevel,
    pub log_message: String,
}

impl From<inventory::NewJobExecutionLog> for NewJobExecutionLog {
    fn from(
        inventory::NewJobExecutionLog {
            execution_id,
            logged_at,
            log_level,
            log_message,
        }: inventory::NewJobExecutionLog,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            logged_at: logged_at.into(),
            log_level: log_level.into(),
            log_message,
        }
    }
}

impl NewJobExecutionLog {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<JobExecutionLog> {
        let job_execution = self
            .insert_into(job_execution_logs::table)
            .get_result(conn)
            .await?;

        Ok(job_execution)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn insert_batch(conn: &mut DbConnection, batch: &[Self]) -> Result<()> {
        // todo: is there a maximum amount of rows for batch inserts?
        batch
            .insert_into(job_execution_logs::table)
            .execute(conn)
            .await?;

        Ok(())
    }
}
