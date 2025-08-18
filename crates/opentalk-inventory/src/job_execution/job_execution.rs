// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::time::Timestamp;

use super::{JobExecutionId, JobId, JobStatus};

/// The representation of a maintenace job execution in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecution {
    /// The id of the job execution.
    pub id: JobExecutionId,

    /// The id of the job.
    pub job_id: JobId,

    /// The timestamp when the job execution was started.
    pub started_at: Timestamp,

    /// An optional timestamp when the job execution ended.
    pub ended_at: Option<Timestamp>,

    /// The status of the job execution.
    pub job_status: JobStatus,
}

impl From<opentalk_db_storage::jobs::JobExecution> for JobExecution {
    fn from(
        opentalk_db_storage::jobs::JobExecution {
            id,
            job_id,
            started_at,
            ended_at,
            job_status,
        }: opentalk_db_storage::jobs::JobExecution,
    ) -> Self {
        Self {
            id: id.into(),
            job_id: job_id.into(),
            started_at: started_at.into(),
            ended_at: ended_at.map(Into::into),
            job_status: job_status.into(),
        }
    }
}

impl From<JobExecution> for opentalk_db_storage::jobs::JobExecution {
    fn from(
        JobExecution {
            id,
            job_id,
            started_at,
            ended_at,
            job_status,
        }: JobExecution,
    ) -> Self {
        Self {
            id: id.into(),
            job_id: job_id.into(),
            started_at: started_at.into(),
            ended_at: ended_at.map(Into::into),
            job_status: job_status.into(),
        }
    }
}
