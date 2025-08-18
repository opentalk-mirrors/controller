// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::time::Timestamp;

use super::JobStatus;

/// Representation of an update to a job execution in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateJobExecution {
    /// An optional timestamp when the job execution ended.
    pub ended_at: Option<Timestamp>,

    /// The status of the job execution.
    pub job_status: Option<JobStatus>,
}

impl From<opentalk_db_storage::jobs::UpdateJobExecution> for UpdateJobExecution {
    fn from(
        opentalk_db_storage::jobs::UpdateJobExecution {
            ended_at,
            job_status,
        }: opentalk_db_storage::jobs::UpdateJobExecution,
    ) -> Self {
        Self {
            ended_at: ended_at.map(Into::into),
            job_status: job_status.map(Into::into),
        }
    }
}

impl From<UpdateJobExecution> for opentalk_db_storage::jobs::UpdateJobExecution {
    fn from(
        UpdateJobExecution {
            ended_at,
            job_status,
        }: UpdateJobExecution,
    ) -> Self {
        Self {
            ended_at: ended_at.map(Into::into),
            job_status: job_status.map(Into::into),
        }
    }
}
