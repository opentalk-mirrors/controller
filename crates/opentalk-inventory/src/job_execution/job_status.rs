// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The status of a maintenance job.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    strum::AsRefStr,
    strum::Display,
    strum::EnumCount,
    strum::EnumIter,
    strum::EnumString,
    strum::VariantNames,
    strum::IntoStaticStr,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// The job has been started.
    Started,

    /// The job run has succeeded.
    Succeeded,

    /// The job has failed.
    Failed,
}

impl From<opentalk_db_storage::jobs::JobStatus> for JobStatus {
    fn from(value: opentalk_db_storage::jobs::JobStatus) -> Self {
        use opentalk_db_storage::jobs::JobStatus as Other;
        match value {
            Other::Started => Self::Started,
            Other::Succeeded => Self::Succeeded,
            Other::Failed => Self::Failed,
        }
    }
}

impl From<JobStatus> for opentalk_db_storage::jobs::JobStatus {
    fn from(value: JobStatus) -> Self {
        use JobStatus as Other;
        match value {
            Other::Started => Self::Started,
            Other::Succeeded => Self::Succeeded,
            Other::Failed => Self::Failed,
        }
    }
}
