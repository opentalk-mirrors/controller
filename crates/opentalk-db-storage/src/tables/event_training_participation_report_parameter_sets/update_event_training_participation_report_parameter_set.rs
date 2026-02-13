// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::events::EventId;

use crate::{
    newtypes::Duration, schema::event_training_participation_report_parameter_sets,
    tables::event_training_participation_report_parameter_sets::EventTrainingParticipationReportParameterSet,
};

#[derive(AsChangeset)]
#[diesel(table_name = event_training_participation_report_parameter_sets)]
pub struct UpdateEventTrainingParticipationReportParameterSet {
    pub initial_checkpoint_delay_after: Option<Duration>,
    pub initial_checkpoint_delay_within: Option<Duration>,
    pub checkpoint_interval_after: Option<Duration>,
    pub checkpoint_interval_within: Option<Duration>,
}

impl From<inventory::UpdateEventTrainingParticipationReportParameterSet>
    for UpdateEventTrainingParticipationReportParameterSet
{
    fn from(
        inventory::UpdateEventTrainingParticipationReportParameterSet {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: inventory::UpdateEventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            initial_checkpoint_delay_after: initial_checkpoint_delay_after.map(Into::into),
            initial_checkpoint_delay_within: initial_checkpoint_delay_within.map(Into::into),
            checkpoint_interval_after: checkpoint_interval_after.map(Into::into),
            checkpoint_interval_within: checkpoint_interval_within.map(Into::into),
        }
    }
}

impl UpdateEventTrainingParticipationReportParameterSet {
    /// Apply the update to the invite where `user_id` is the invitee
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        event_id: EventId,
    ) -> Result<EventTrainingParticipationReportParameterSet> {
        let query = diesel::update(event_training_participation_report_parameter_sets::table)
            .filter(event_training_participation_report_parameter_sets::event_id.eq(event_id))
            .set(self)
            // change it here
            .returning(event_training_participation_report_parameter_sets::all_columns);

        let event_training_participation_report_parameter_sets = query.get_result(conn).await?;

        Ok(event_training_participation_report_parameter_sets)
    }
}
