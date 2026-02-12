// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, Queryable,
    expression::AsExpression, prelude::*,
};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{
        EventId,
        invites::{EventInviteStatus, InviteRole},
    },
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    training_participation_report::TimeRange,
    users::UserId,
};
use serde::{Deserialize, Serialize};

use crate::{
    newtypes::Duration,
    paginate::Paginate,
    schema::{
        event_favorites, event_invites, event_training_participation_report_parameter_sets, events,
        users,
    },
    users::User,
};

pub use crate::tables::{
    event_exceptions::{EventException, NewEventException, UpdateEventException},
    events::{Event, NewEvent, UpdateEvent},
}; // TODO: rm -f

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
#[diesel(sql_type = diesel::sql_types::Uuid)]
pub struct EventInviteId(uuid::Uuid);

impl From<inventory::EventInviteId> for EventInviteId {
    fn from(value: inventory::EventInviteId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}

impl From<EventInviteId> for inventory::EventInviteId {
    fn from(value: EventInviteId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}

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

#[derive(Debug, Queryable, Identifiable, Associations)]
#[diesel(table_name = event_invites)]
#[diesel(belongs_to(Event, foreign_key = event_id))]
#[diesel(belongs_to(User, foreign_key = invitee))]
pub struct EventInvite {
    pub id: EventInviteId,
    pub event_id: EventId,
    pub invitee: UserId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub status: EventInviteStatus,
    pub role: InviteRole,
}

impl From<EventInvite> for inventory::EventInvite {
    fn from(
        EventInvite {
            id,
            event_id,
            invitee,
            created_by,
            created_at,
            status,
            role,
        }: EventInvite,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            invitee,
            created_by,
            created_at: created_at.into(),
            status,
            role,
        }
    }
}

impl From<inventory::EventInvite> for EventInvite {
    fn from(
        inventory::EventInvite {
            id,
            event_id,
            invitee,
            created_by,
            created_at,
            status,
            role,
        }: inventory::EventInvite,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            invitee,
            created_by,
            created_at: created_at.into(),
            status,
            role,
        }
    }
}

impl EventInvite {
    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_events(
        conn: &mut DbConnection,
        events: &[&Event],
    ) -> Result<Vec<Vec<(EventInvite, User)>>> {
        conn.transaction(|conn| {
            async move {
                let invites: Vec<EventInvite> =
                    EventInvite::belonging_to(events).load(conn).await?;
                let mut user_ids: Vec<UserId> = invites.iter().map(|x| x.invitee).collect();
                // Small optimization to filter out duplicates
                user_ids.sort_unstable();
                user_ids.dedup();

                let users = User::get_all_by_ids(conn, &user_ids).await?;

                let invites_by_event: Vec<Vec<EventInvite>> = invites.grouped_by(events);
                let mut invites_with_users_by_event = Vec::with_capacity(events.len());

                for invites in invites_by_event {
                    let mut invites_with_users = Vec::with_capacity(invites.len());

                    for invite in invites {
                        let user = users
                            .iter()
                            .find(|user| user.id == invite.invitee)
                            .ok_or_else(|| DatabaseError::Custom {
                                message: "bug: user invite invitee missing".to_owned(),
                            })?;

                        invites_with_users.push((invite, user.clone()))
                    }

                    invites_with_users_by_event.push(invites_with_users);
                }

                Ok(invites_with_users_by_event)
            }
            .scope_boxed()
        })
        .await
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_event_paginated(
        conn: &mut DbConnection,
        event_id: EventId,
        per_page: PageSize,
        page: Page,
        filter_by_status: Option<EventInviteStatus>,
    ) -> Result<(Vec<(EventInvite, User)>, ItemCount)> {
        let allowed_states = filter_by_status
            .map(|s| BTreeSet::from([s]))
            .unwrap_or_else(EventInviteStatus::all_enum_values);

        let query = event_invites::table
            .inner_join(users::table.on(event_invites::invitee.eq(users::id)))
            .filter(
                event_invites::columns::event_id
                    .eq(event_id)
                    .and(event_invites::columns::status.eq_any(allowed_states)),
            )
            .order(event_invites::created_at.desc())
            .then_order_by(event_invites::created_by.desc())
            .then_order_by(event_invites::invitee.desc())
            .paginate_by(per_page, page);

        let invites = query.load_and_count(conn).await?;

        Ok(invites)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_pending_for_user(
        conn: &mut DbConnection,
        user_id: UserId,
    ) -> Result<Vec<EventInvite>> {
        let query = event_invites::table.filter(
            event_invites::invitee
                .eq(user_id)
                .and(event_invites::status.eq(EventInviteStatus::Pending)),
        );

        let event_invites = query.load(conn).await?;

        Ok(event_invites)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_user_and_room(
        conn: &mut DbConnection,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Option<EventInvite>> {
        let query = event_invites::table
            .select(event_invites::all_columns)
            .inner_join(
                events::table.on(events::id
                    .eq(event_invites::event_id)
                    .and(events::room.eq(room_id))),
            )
            .filter(event_invites::invitee.eq(user_id));

        let event_invite = query.first(conn).await.optional()?;

        Ok(event_invite)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_invitee(
        conn: &mut DbConnection,
        event_id: EventId,
        invitee: UserId,
    ) -> Result<EventInvite> {
        let query = diesel::delete(event_invites::table)
            .filter(
                event_invites::event_id
                    .eq(event_id)
                    .and(event_invites::invitee.eq(invitee)),
            )
            .returning(event_invites::all_columns);

        let event_invite = query.get_result(conn).await?;

        Ok(event_invite)
    }
}

#[derive(Insertable)]
#[diesel(table_name = event_invites)]
pub struct NewEventInvite {
    pub event_id: EventId,
    pub invitee: UserId,
    pub role: InviteRole,
    pub created_by: UserId,
    pub created_at: Option<DateTime<Utc>>,
}

impl From<inventory::NewEventInvite> for NewEventInvite {
    fn from(
        inventory::NewEventInvite {
            event_id,
            invitee,
            role,
            created_by,
            created_at,
        }: inventory::NewEventInvite,
    ) -> Self {
        Self {
            event_id,
            invitee,
            role,
            created_by,
            created_at: created_at.map(Into::into),
        }
    }
}

impl NewEventInvite {
    /// Tries to insert the EventInvite into the database
    ///
    /// When yielding a unique key violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<EventInvite>> {
        let query = self.insert_into(event_invites::table);

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
}

#[derive(AsChangeset)]
#[diesel(table_name = event_invites)]
pub struct UpdateEventInvite {
    pub status: Option<EventInviteStatus>,
    pub role: Option<InviteRole>,
}

impl From<inventory::UpdateEventInvite> for UpdateEventInvite {
    fn from(inventory::UpdateEventInvite { status, role }: inventory::UpdateEventInvite) -> Self {
        Self { status, role }
    }
}

impl UpdateEventInvite {
    /// Apply the update to the invite where `user_id` is the invitee
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        user_id: UserId,
        event_id: EventId,
    ) -> Result<EventInvite> {
        // TODO: Check if the update actually applied a change (also have a look at fn `apply` of `UpdateEventEmailInvite`)
        // Use something like
        // UPDATE event_invites SET status = $status WHERE id = $id RETURNING id, status, (SELECT status FROM tmp WHERE id = $id);
        // or
        // UPDATE event_invites SET status = $status WHERE id = $id FROM event_invites old RETURNING old.*;
        // and compare the value to the set one to return if the value was changed
        let query = diesel::update(event_invites::table)
            .filter(
                event_invites::event_id
                    .eq(event_id)
                    .and(event_invites::invitee.eq(user_id)),
            )
            .set(self)
            // change it here
            .returning(event_invites::all_columns);

        let event_invite = query.get_result(conn).await?;

        Ok(event_invite)
    }
}

#[derive(Associations, Identifiable, Queryable)]
#[diesel(table_name = event_favorites)]
#[diesel(primary_key(user_id, event_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Event))]
pub struct EventFavorite {
    pub user_id: UserId,
    pub event_id: EventId,
}

impl EventFavorite {
    /// Deletes a EventFavorite entry by user_id and event_id
    ///
    /// Returns true if something was deleted
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(
        conn: &mut DbConnection,
        user_id: UserId,
        event_id: EventId,
    ) -> Result<bool> {
        let lines_changes = diesel::delete(event_favorites::table)
            .filter(
                event_favorites::user_id
                    .eq(user_id)
                    .and(event_favorites::event_id.eq(event_id)),
            )
            .execute(conn)
            .await?;

        Ok(lines_changes > 0)
    }
}

#[derive(Insertable)]
#[diesel(table_name = event_favorites)]
pub struct NewEventFavorite {
    pub user_id: UserId,
    pub event_id: EventId,
}

impl NewEventFavorite {
    /// Tries to insert the NewEventFavorite into the database
    ///
    /// When yielding a unique key violation, None is returned.
    #[tracing::instrument(err, skip_all)]
    pub async fn try_insert(self, conn: &mut DbConnection) -> Result<Option<EventFavorite>> {
        let query = self.insert_into(event_favorites::table);

        let result = query.get_result(conn).await;

        match result {
            Ok(event_favorite) => Ok(Some(event_favorite)),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                ..,
            )) => Ok(None),
            Err(e) => Err(e.into()),
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
