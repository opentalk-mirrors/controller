// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::jobs::{self as db};
use opentalk_inventory::{
    Job, JobExecution, JobExecutionId, JobExecutionInventory, JobId, NewJobExecution,
    NewJobExecutionLog, UpdateJobExecution, error::StorageBackendSnafu,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl JobExecutionInventory for DatabaseConnection {
    async fn get_job(&mut self, job_id: JobId) -> Result<Job> {
        Ok(db::Job::get(&mut self.inner, job_id.into())
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    async fn get_all_jobs(&mut self) -> Result<Vec<Job>> {
        Ok(db::Job::get_all(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn update_job_execution(
        &mut self,
        job_execution_id: JobExecutionId,
        job_execution: UpdateJobExecution,
    ) -> Result<JobExecution> {
        Ok(db::UpdateJobExecution::from(job_execution)
            .apply(&mut self.inner, job_execution_id.into())
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    async fn create_job_execution(
        &mut self,
        job_execution: NewJobExecution,
    ) -> Result<JobExecution> {
        Ok(db::NewJobExecution::from(job_execution)
            .insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    /// Create a new batch of job execution logs.
    async fn create_job_execution_logs(
        &mut self,
        job_execution_logs: &[NewJobExecutionLog],
    ) -> Result<()> {
        // TODO: this has much worse performance than it should,
        // we should get rid of the clone and perform the insertion directly.
        let job_execution_logs: Vec<_> = job_execution_logs
            .iter()
            .cloned()
            .map(db::NewJobExecutionLog::from)
            .collect();
        db::NewJobExecutionLog::insert_batch(&mut self.inner, &job_execution_logs)
            .await
            .context(StorageBackendSnafu)
    }
}
