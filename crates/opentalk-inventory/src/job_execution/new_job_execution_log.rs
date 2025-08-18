// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::time::Timestamp;

use super::{JobExecutionId, JobExecutionLogLevel};

/// The representation of a new job execution log that is intended to be stored in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewJobExecutionLog {
    /// The id of the job execution.
    pub execution_id: JobExecutionId,

    /// The logging timestamp.
    pub logged_at: Timestamp,

    /// The log level.
    pub log_level: JobExecutionLogLevel,

    /// The log message.
    pub log_message: String,
}

impl From<opentalk_db_storage::jobs::NewJobExecutionLog> for NewJobExecutionLog {
    fn from(
        opentalk_db_storage::jobs::NewJobExecutionLog {
            execution_id,
            logged_at,
            log_level,
            log_message,
        }: opentalk_db_storage::jobs::NewJobExecutionLog,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            logged_at: logged_at.into(),
            log_level: log_level.into(),
            log_message,
        }
    }
}

impl From<NewJobExecutionLog> for opentalk_db_storage::jobs::NewJobExecutionLog {
    fn from(
        NewJobExecutionLog {
            execution_id,
            logged_at,
            log_level,
            log_message,
        }: NewJobExecutionLog,
    ) -> Self {
        Self {
            execution_id: execution_id.into(),
            logged_at: logged_at.into(),
            log_level: log_level.into(),
            log_message,
        }
    }
}
