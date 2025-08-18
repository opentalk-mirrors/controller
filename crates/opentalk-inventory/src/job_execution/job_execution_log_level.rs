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
pub enum JobExecutionLogLevel {
    /// TRACE log level.
    Trace,

    /// DEBUG log level.
    Debug,

    /// INFO log level.
    Info,

    /// WARN log level.
    Warn,

    /// ERROR log level.
    Error,
}

impl From<opentalk_db_storage::jobs::LogLevel> for JobExecutionLogLevel {
    fn from(value: opentalk_db_storage::jobs::LogLevel) -> Self {
        use opentalk_db_storage::jobs::LogLevel as Other;
        match value {
            Other::Trace => Self::Trace,
            Other::Debug => Self::Debug,
            Other::Info => Self::Info,
            Other::Warn => Self::Warn,
            Other::Error => Self::Error,
        }
    }
}

impl From<JobExecutionLogLevel> for opentalk_db_storage::jobs::LogLevel {
    fn from(value: JobExecutionLogLevel) -> Self {
        use JobExecutionLogLevel as Other;
        match value {
            Other::Trace => Self::Trace,
            Other::Debug => Self::Debug,
            Other::Info => Self::Info,
            Other::Warn => Self::Warn,
            Other::Error => Self::Error,
        }
    }
}
