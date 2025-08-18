// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The id of a maintenance job that is stored in the inventory.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
)]
pub struct JobExecutionId(i64);

impl From<opentalk_db_storage::jobs::SerialId> for JobExecutionId {
    fn from(value: opentalk_db_storage::jobs::SerialId) -> Self {
        Self(value.into())
    }
}

impl From<JobExecutionId> for opentalk_db_storage::jobs::SerialId {
    fn from(JobExecutionId(value): JobExecutionId) -> Self {
        Self::from(value)
    }
}
