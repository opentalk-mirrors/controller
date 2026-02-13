// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Insertable, OptionalExtension, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{events::EventId, training_participation_report::TimeRange};

use crate::{newtypes::Duration, schema::event_training_participation_report_parameter_sets};

pub use crate::tables::{
    event_exceptions::{EventException, NewEventException, UpdateEventException},
    event_favorites::{EventFavorite, NewEventFavorite},
    event_invites::{EventInvite, NewEventInvite, UpdateEventInvite},
    events::{Event, NewEvent, UpdateEvent},
}; // TODO: rm -f

pub mod email_invites;
pub mod shared_folders;

pub struct GetEventsCursor {
    pub from_id: EventId,
    pub from_created_at: DateTime<Utc>,
    pub from_starts_at: Option<DateTime<Utc>>,
}

impl From<GetEventsCursor> for inventory::GetEventsCursor {
    fn from(
        GetEventsCursor {
            from_id,
            from_created_at,
            from_starts_at,
        }: GetEventsCursor,
    ) -> Self {
        Self::new(
            from_id,
            from_created_at.into(),
            from_starts_at.map(Into::into),
        )
    }
}

impl From<inventory::GetEventsCursor> for GetEventsCursor {
    fn from(value: inventory::GetEventsCursor) -> Self {
        let (from_id, from_created_at, from_starts_at) = value.into();
        Self {
            from_id,
            from_created_at: from_created_at.into(),
            from_starts_at: from_starts_at.map(Into::into),
        }
    }
}

impl GetEventsCursor {
    pub fn from_last_event_in_query(event: &Event) -> Self {
        Self {
            from_id: event.id,
            from_created_at: event.created_at,
            from_starts_at: event.starts_at,
        }
    }
}

pub struct GetEventExceptionsCursor {
    pub from_id: EventId,
    pub from_created_at: DateTime<Utc>,
    pub from_starts_at: Option<DateTime<Utc>>,
    pub from_exception_date: DateTime<Utc>,
}

impl From<GetEventExceptionsCursor> for inventory::GetEventExceptionsCursor {
    fn from(
        GetEventExceptionsCursor {
            from_id,
            from_created_at,
            from_starts_at,
            from_exception_date,
        }: GetEventExceptionsCursor,
    ) -> Self {
        Self::new(
            from_id,
            from_created_at.into(),
            from_starts_at.map(Into::into),
            from_exception_date.into(),
        )
    }
}

impl From<inventory::GetEventExceptionsCursor> for GetEventExceptionsCursor {
    fn from(value: inventory::GetEventExceptionsCursor) -> Self {
        let (from_id, from_created_at, from_starts_at, from_exception_date) = value.into();
        Self {
            from_id,
            from_created_at: from_created_at.into(),
            from_starts_at: from_starts_at.map(Into::into),
            from_exception_date: from_exception_date.into(),
        }
    }
}

impl GetEventExceptionsCursor {
    pub fn from_last_event_in_query(exception: &EventException) -> Self {
        Self {
            from_id: exception.event_id,
            from_created_at: exception.created_at,
            from_starts_at: exception.starts_at,
            from_exception_date: exception.exception_date,
        }
    }
}

#[derive(Debug, Insertable, Queryable, Identifiable, Associations)]
#[diesel(table_name = event_training_participation_report_parameter_sets)]
#[diesel(primary_key(event_id))]
#[diesel(belongs_to(Event, foreign_key = event_id))]
pub struct EventTrainingParticipationReportParameterSet {
    pub event_id: EventId,
    pub initial_checkpoint_delay_after: Duration,
    pub initial_checkpoint_delay_within: Duration,
    pub checkpoint_interval_after: Duration,
    pub checkpoint_interval_within: Duration,
}

impl From<EventTrainingParticipationReportParameterSet>
    for inventory::EventTrainingParticipationReportParameterSet
{
    fn from(
        EventTrainingParticipationReportParameterSet {
            event_id,
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: EventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            event_id,
            initial_checkpoint_delay: TimeRange::new_with_clamped_durations(
                initial_checkpoint_delay_after.into(),
                initial_checkpoint_delay_within.into(),
            ),
            checkpoint_interval: TimeRange::new_with_clamped_durations(
                checkpoint_interval_after.into(),
                checkpoint_interval_within.into(),
            ),
        }
    }
}

impl From<inventory::EventTrainingParticipationReportParameterSet>
    for EventTrainingParticipationReportParameterSet
{
    fn from(
        inventory::EventTrainingParticipationReportParameterSet {
            event_id,
            initial_checkpoint_delay,
            checkpoint_interval,
        }: inventory::EventTrainingParticipationReportParameterSet,
    ) -> Self {
        Self {
            event_id,
            initial_checkpoint_delay_after: initial_checkpoint_delay.after().into(),
            initial_checkpoint_delay_within: initial_checkpoint_delay.within().into(),
            checkpoint_interval_after: checkpoint_interval.after().into(),
            checkpoint_interval_within: checkpoint_interval.within().into(),
        }
    }
}

impl EventTrainingParticipationReportParameterSet {
    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_event(conn: &mut DbConnection, event_id: EventId) -> Result<Option<Self>> {
        let parameter_set = event_training_participation_report_parameter_sets::table
            .filter(event_training_participation_report_parameter_sets::event_id.eq(event_id))
            .get_result(conn)
            .await
            .optional()?;

        Ok(parameter_set)
    }

    /// Tries to insert the EventTrainingParticipationParameterSet into the database
    ///
    /// When yielding a unique key violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<Self>> {
        let query = self.insert_into(event_training_participation_report_parameter_sets::table);

        let result = query.get_result(conn).await;

        match result {
            Ok(event_invite) => Ok(Some(event_invite)),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..,
            )) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(conn: &mut DbConnection, event_id: EventId) -> Result<()> {
        let query = diesel::delete(
            event_training_participation_report_parameter_sets::table
                .filter(event_training_participation_report_parameter_sets::event_id.eq(event_id)),
        );

        query.execute(conn).await?;

        Ok(())
    }
}

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
