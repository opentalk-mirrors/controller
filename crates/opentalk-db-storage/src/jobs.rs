// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::{ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_types_common::sql_enum;
use serde::{Deserialize, Serialize};

use crate::schema::{job_execution_logs, job_executions, jobs};

#[derive(
    AsRef,
    Display,
    From,
    FromStr,
    Into,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsExpression,
    FromSqlRow,
    DieselNewtype,
)]
#[diesel(sql_type = diesel::sql_types::BigInt)]
pub struct SerialId(i64);

impl From<SerialId> for opentalk_inventory::JobId {
    fn from(SerialId(value): SerialId) -> Self {
        Self::from(value)
    }
}

impl From<opentalk_inventory::JobId> for SerialId {
    fn from(value: opentalk_inventory::JobId) -> Self {
        Self(value.into())
    }
}

impl From<SerialId> for opentalk_inventory::JobExecutionId {
    fn from(SerialId(value): SerialId) -> Self {
        Self::from(value)
    }
}

impl From<opentalk_inventory::JobExecutionId> for SerialId {
    fn from(value: opentalk_inventory::JobExecutionId) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
pub struct Job {
    pub id: SerialId,
    pub name: String,
    pub kind: JobType,
    pub parameters: serde_json::Value,
    pub timeout_secs: i32,
    pub recurrence: String,
}

impl From<Job> for opentalk_inventory::Job {
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

impl From<opentalk_inventory::Job> for Job {
    fn from(
        opentalk_inventory::Job {
            id,
            name,
            kind,
            parameters,
            timeout_secs,
            recurrence,
        }: opentalk_inventory::Job,
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
    pub async fn get(conn: &mut DbConnection, id: SerialId) -> Result<Self> {
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

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
pub struct JobExecution {
    pub id: SerialId,
    pub job_id: SerialId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub job_status: JobStatus,
}

impl From<JobExecution> for opentalk_inventory::JobExecution {
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

impl From<opentalk_inventory::JobExecution> for JobExecution {
    fn from(
        opentalk_inventory::JobExecution {
            id,
            job_id,
            started_at,
            ended_at,
            job_status,
        }: opentalk_inventory::JobExecution,
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
    pub job_id: SerialId,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub job_status: JobStatus,
}

impl From<opentalk_inventory::NewJobExecution> for NewJobExecution {
    fn from(
        opentalk_inventory::NewJobExecution {
            job_id,
            started_at,
            ended_at,
            job_status,
        }: opentalk_inventory::NewJobExecution,
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

impl From<opentalk_inventory::UpdateJobExecution> for UpdateJobExecution {
    fn from(
        opentalk_inventory::UpdateJobExecution {
            ended_at,
            job_status,
        }: opentalk_inventory::UpdateJobExecution,
    ) -> Self {
        Self {
            ended_at: ended_at.map(Into::into),
            job_status: job_status.map(Into::into),
        }
    }
}

impl UpdateJobExecution {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(self, conn: &mut DbConnection, id: SerialId) -> Result<JobExecution> {
        let target = job_executions::table.filter(job_executions::id.eq(&id));
        let job_execution = diesel::update(target).set(self).get_result(conn).await?;

        Ok(job_execution)
    }
}

#[derive(Debug, Clone, Queryable, Identifiable, PartialEq, Eq)]
pub struct JobExecutionLog {
    pub id: SerialId,
    pub execution_id: SerialId,
    pub logged_at: DateTime<Utc>,
    pub log_level: LogLevel,
    pub log_message: String,
}

impl JobExecutionLog {}

#[derive(Debug, Insertable)]
#[diesel(table_name = job_execution_logs)]
pub struct NewJobExecutionLog {
    pub execution_id: SerialId,
    pub logged_at: DateTime<Utc>,
    pub log_level: LogLevel,
    pub log_message: String,
}

impl From<opentalk_inventory::NewJobExecutionLog> for NewJobExecutionLog {
    fn from(
        opentalk_inventory::NewJobExecutionLog {
            execution_id,
            logged_at,
            log_level,
            log_message,
        }: opentalk_inventory::NewJobExecutionLog,
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
    JobType,
    "job_type",
    JobTypeType,
    {
        AdhocEventCleanup = b"adhoc_event_cleanup",
        EventCleanup = b"event_cleanup",
        InviteCleanup = b"invite_cleanup",
        SelfCheck = b"self_check",
        SyncStorageFiles = b"sync_storage_files",
        RoomCleanup = b"room_cleanup",
        KeycloakAccountSync = b"keycloak_account_sync",
        UserCleanup = b"user_cleanup"
    }
);

impl From<JobType> for opentalk_inventory::JobType {
    fn from(value: JobType) -> Self {
        match value {
            JobType::AdhocEventCleanup => Self::AdhocEventCleanup,
            JobType::EventCleanup => Self::EventCleanup,
            JobType::UserCleanup => Self::UserCleanup,
            JobType::InviteCleanup => Self::InviteCleanup,
            JobType::SelfCheck => Self::SelfCheck,
            JobType::SyncStorageFiles => Self::SyncStorageFiles,
            JobType::RoomCleanup => Self::RoomCleanup,
            JobType::KeycloakAccountSync => Self::KeycloakAccountSync,
        }
    }
}

impl From<opentalk_inventory::JobType> for JobType {
    fn from(value: opentalk_inventory::JobType) -> Self {
        use opentalk_inventory::JobType as Other;
        match value {
            Other::AdhocEventCleanup => Self::AdhocEventCleanup,
            Other::EventCleanup => Self::EventCleanup,
            Other::UserCleanup => Self::UserCleanup,
            Other::InviteCleanup => Self::InviteCleanup,
            Other::SelfCheck => Self::SelfCheck,
            Other::SyncStorageFiles => Self::SyncStorageFiles,
            Other::RoomCleanup => Self::RoomCleanup,
            Other::KeycloakAccountSync => Self::KeycloakAccountSync,
        }
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

impl From<JobStatus> for opentalk_inventory::JobStatus {
    fn from(value: JobStatus) -> Self {
        match value {
            JobStatus::Started => Self::Started,
            JobStatus::Succeeded => Self::Succeeded,
            JobStatus::Failed => Self::Failed,
        }
    }
}

impl From<opentalk_inventory::JobStatus> for JobStatus {
    fn from(value: opentalk_inventory::JobStatus) -> Self {
        use opentalk_inventory::JobStatus as Other;
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

impl From<LogLevel> for opentalk_inventory::JobExecutionLogLevel {
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

impl From<opentalk_inventory::JobExecutionLogLevel> for LogLevel {
    fn from(value: opentalk_inventory::JobExecutionLogLevel) -> Self {
        use opentalk_inventory::JobExecutionLogLevel as Other;
        match value {
            Other::Trace => Self::Trace,
            Other::Debug => Self::Debug,
            Other::Info => Self::Info,
            Other::Warn => Self::Warn,
            Other::Error => Self::Error,
        }
    }
}
