// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use derive_more::Display;
use diesel::{ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::sql_enum;

use crate::schema::{job_execution_logs, job_executions};

pub use crate::tables::jobs::{Job, JobTypeType, SerialJobId};

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
#[diesel(table_name = job_executions)]
pub struct JobExecution {
    pub id: SerialJobId,
    pub job_id: SerialJobId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub job_status: JobStatus,
}

impl From<JobExecution> for inventory::JobExecution {
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

impl From<inventory::JobExecution> for JobExecution {
    fn from(
        inventory::JobExecution {
            id,
            job_id,
            started_at,
            ended_at,
            job_status,
        }: inventory::JobExecution,
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

#[derive(Debug, Insertable)]
#[diesel(table_name = job_executions)]
pub struct NewJobExecution {
    pub job_id: SerialJobId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub job_status: JobStatus,
}

impl From<inventory::NewJobExecution> for NewJobExecution {
    fn from(
        inventory::NewJobExecution {
            job_id,
            started_at,
            ended_at,
            job_status,
        }: inventory::NewJobExecution,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            started_at: started_at.into(),
            ended_at: ended_at.map(Into::into),
            job_status: job_status.into(),
        }
    }
}

impl NewJobExecution {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<JobExecution> {
        let job_execution = self
            .insert_into(job_executions::table)
            .get_result(conn)
            .await?;

        Ok(job_execution)
    }
}

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

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
#[diesel(table_name = job_execution_logs)]
pub struct JobExecutionLog {
    pub id: SerialJobId,
    pub execution_id: SerialJobId,
    pub logged_at: DateTime<Utc>,
    pub log_level: LogLevel,
    pub log_message: String,
}

impl JobExecutionLog {}

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

sql_enum!(
    #[derive(PartialEq, Eq, Display)]
    JobStatus,
    "job_status",
    JobStatusType,
    {
        Started = b"started",
        Succeeded = b"succeeded",
        Failed = b"failed",
    }
);

impl From<JobStatus> for inventory::JobStatus {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Started => Self::Started,
            JobStatus::Succeeded => Self::Succeeded,
            JobStatus::Failed => Self::Failed,
        }
    }
}

impl From<inventory::JobStatus> for JobStatus {
    fn from(value: inventory::JobStatus) -> Self {
        use inventory::JobStatus as Other;
        match value {
            Other::Started => Self::Started,
            Other::Succeeded => Self::Succeeded,
            Other::Failed => Self::Failed,
        }
    }
}

sql_enum!(
    #[derive(PartialEq, Eq, Display)]
    LogLevel,
    "log_level",
    LogLevelType,
    {
        Trace= b"trace",
        Debug = b"debug",
        Info = b"info",
        Warn = b"warn",
        Error = b"error",
    }
);

impl From<LogLevel> for inventory::JobExecutionLogLevel {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => Self::Trace,
            LogLevel::Debug => Self::Debug,
            LogLevel::Info => Self::Info,
            LogLevel::Warn => Self::Warn,
            LogLevel::Error => Self::Error,
        }
    }
}

impl From<inventory::JobExecutionLogLevel> for LogLevel {
    fn from(value: inventory::JobExecutionLogLevel) -> Self {
        use inventory::JobExecutionLogLevel as Other;
        match value {
            Other::Trace => Self::Trace,
            Other::Debug => Self::Debug,
            Other::Info => Self::Info,
            Other::Warn => Self::Warn,
            Other::Error => Self::Error,
        }
    }
}
