// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use super::JobType;

/// The representation of a maintenace job in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The id of the job.
    pub id: i64,

    /// The name of the job.
    pub name: String,

    /// The kind of job.
    pub kind: JobType,

    /// The parameters passed to the job when it is executed.
    pub parameters: serde_json::Value,

    /// The timeout duration.
    pub timeout_secs: i32,

    /// The recurrence pattern for the job.
    pub recurrence: String,
}

impl From<opentalk_db_storage::jobs::Job> for Job {
    fn from(
        opentalk_db_storage::jobs::Job {
            id,
            name,
            kind,
            parameters,
            timeout_secs,
            recurrence,
        }: opentalk_db_storage::jobs::Job,
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

impl From<Job> for opentalk_db_storage::jobs::Job {
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
