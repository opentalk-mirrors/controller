// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::time::Timestamp;

use super::{JobId, JobStatus};

/// The representation of a new job execution that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewJobExecution {
    /// The id of the job.
    pub job_id: JobId,

    /// The timestamp when the job execution was started.
    pub started_at: Timestamp,

    /// An optional timestamp when the job execution ended.
    pub ended_at: Option<Timestamp>,

    /// The status of the job execution.
    pub job_status: JobStatus,
}

impl From<opentalk_db_storage::jobs::NewJobExecution> for NewJobExecution {
    fn from(
        opentalk_db_storage::jobs::NewJobExecution {
            job_id,
            started_at,
            ended_at,
            job_status,
        }: opentalk_db_storage::jobs::NewJobExecution,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            started_at: started_at.into(),
            ended_at: ended_at.map(Into::into),
            job_status: job_status.into(),
        }
    }
}

impl From<NewJobExecution> for opentalk_db_storage::jobs::NewJobExecution {
    fn from(
        NewJobExecution {
            job_id,
            started_at,
            ended_at,
            job_status,
        }: NewJobExecution,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            started_at: started_at.into(),
            ended_at: ended_at.map(Into::into),
            job_status: job_status.into(),
        }
    }
}
