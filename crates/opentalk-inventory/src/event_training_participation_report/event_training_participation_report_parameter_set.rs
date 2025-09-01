// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_common::{
    events::EventId,
    training_participation_report::{
        TimeRange, TimeRangeStart, TimeRangeWindow, TrainingParticipationReportParameterSet,
    },
};

/// The representation of a parameter set for the training participation report in the inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTrainingParticipationReportParameterSet {
    /// The id of the event.
    pub event_id: EventId,

    /// The minimum duration until the initial checkpoint.
    pub initial_checkpoint_delay_after: i64,

    /// The timespan within which the initial checkpoint happens.
    pub initial_checkpoint_delay_within: i64,

    /// The minimum duration for the checkpoint interval.
    pub checkpoint_interval_after: i64,

    /// The timespan within which the checkpoint interval happens.
    pub checkpoint_interval_within: i64,
}

impl From<EventTrainingParticipationReportParameterSet>
    for TrainingParticipationReportParameterSet
{
    fn from(
        EventTrainingParticipationReportParameterSet {
            event_id: _,
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: EventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            initial_checkpoint_delay: TimeRange {
                after: TimeRangeStart::try_from(initial_checkpoint_delay_after).unwrap_or_default(),
                within: TimeRangeWindow::try_from(initial_checkpoint_delay_within)
                    .unwrap_or_default(),
            },
            checkpoint_interval: TimeRange {
                after: TimeRangeStart::try_from(checkpoint_interval_after).unwrap_or_default(),
                within: TimeRangeWindow::try_from(checkpoint_interval_within).unwrap_or_default(),
            },
        }
    }
}

impl From<(EventId, TrainingParticipationReportParameterSet)>
    for EventTrainingParticipationReportParameterSet
{
    fn from(
        (
            event_id,
            TrainingParticipationReportParameterSet {
                initial_checkpoint_delay,
                checkpoint_interval,
            },
        ): (EventId, TrainingParticipationReportParameterSet),
    ) -> Self {
        Self {
            event_id,
            initial_checkpoint_delay_after: initial_checkpoint_delay.after.into(),
            initial_checkpoint_delay_within: initial_checkpoint_delay.within.into(),
            checkpoint_interval_after: checkpoint_interval.after.into(),
            checkpoint_interval_within: checkpoint_interval.within.into(),
        }
    }
}
