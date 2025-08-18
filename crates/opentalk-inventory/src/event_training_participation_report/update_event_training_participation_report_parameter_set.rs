// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// Representation of an update to a training participation report parameter set in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateEventTrainingParticipationReportParameterSet {
    /// The minimum duration until the initial checkpoint.
    pub initial_checkpoint_delay_after: Option<i64>,

    /// The timespan within which the initial checkpoint happens.
    pub initial_checkpoint_delay_within: Option<i64>,

    /// The minimum duration for the checkpoint interval.
    pub checkpoint_interval_after: Option<i64>,

    /// The timespan within which the checkpoint interval happens.
    pub checkpoint_interval_within: Option<i64>,
}

impl From<opentalk_db_storage::events::UpdateEventTrainingParticipationReportParameterSet>
    for UpdateEventTrainingParticipationReportParameterSet
{
    fn from(
        opentalk_db_storage::events::UpdateEventTrainingParticipationReportParameterSet {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: opentalk_db_storage::events::UpdateEventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }
    }
}

impl From<UpdateEventTrainingParticipationReportParameterSet>
    for opentalk_db_storage::events::UpdateEventTrainingParticipationReportParameterSet
{
    fn from(
        UpdateEventTrainingParticipationReportParameterSet {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: UpdateEventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }
    }
}
