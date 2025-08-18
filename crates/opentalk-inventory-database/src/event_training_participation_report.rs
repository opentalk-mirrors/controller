// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::events as db;
use opentalk_inventory::{
    EventTrainingParticipationReportInventory, EventTrainingParticipationReportParameterSet,
    UpdateEventTrainingParticipationReportParameterSet, error::StorageBackendSnafu,
};
use opentalk_types_common::events::EventId;
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl EventTrainingParticipationReportInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn get_event_training_participation_report_parameter_set(
        &mut self,
        event_id: EventId,
    ) -> Result<Option<EventTrainingParticipationReportParameterSet>> {
        Ok(
            db::EventTrainingParticipationReportParameterSet::get_for_event(
                &mut self.inner,
                event_id,
            )
            .await
            .context(StorageBackendSnafu)?
            .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_training_participation_report_parameter_set(
        &mut self,
        event_id: EventId,
        parameter_set: UpdateEventTrainingParticipationReportParameterSet,
    ) -> Result<EventTrainingParticipationReportParameterSet> {
        Ok(
            db::UpdateEventTrainingParticipationReportParameterSet::from(parameter_set)
                .apply(&mut self.inner, event_id)
                .await
                .context(StorageBackendSnafu)?
                .into(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn try_create_event_training_participation_report_parameter_set(
        &mut self,
        parameter_set: EventTrainingParticipationReportParameterSet,
    ) -> Result<Option<EventTrainingParticipationReportParameterSet>> {
        Ok(
            db::EventTrainingParticipationReportParameterSet::from(parameter_set)
                .try_insert(&mut self.inner)
                .await
                .context(StorageBackendSnafu)?
                .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_training_participation_report_parameter_set(
        &mut self,
        event_id: EventId,
    ) -> Result<()> {
        db::EventTrainingParticipationReportParameterSet::delete_by_id(&mut self.inner, event_id)
            .await
            .context(StorageBackendSnafu)
    }
}
