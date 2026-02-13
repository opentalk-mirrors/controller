// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    sql_types::{Nullable, Record, Timestamptz, Uuid},
};
use diesel_async::RunQueryDsl;
use futures_core::Stream;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle, invites::EventInviteStatus},
    rooms::RoomId,
    tenants::TenantId,
    time::TimeZone,
    training_participation_report::TrainingParticipationReportParameterSet,
    users::UserId,
};
use redis_args::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use crate::{
    events::{
        EventException, EventInvite, EventTrainingParticipationReportParameterSet,
        shared_folders::EventSharedFolder,
    },
    queries::events::cursor::{GetEventExceptionsCursor, GetEventsCursor},
    rooms::Room,
    schema::{
        event_exceptions, event_favorites, event_invites, event_shared_folders,
        event_training_participation_report_parameter_sets, events, rooms, sip_configs, tariffs,
        users,
    },
    sip_configs::SipConfig,
    tables::events::EventSerialId,
    tariffs::Tariff,
    users::User,
    utils::convert_diesel_query_results,
};

#[derive(
    Associations,
    Clone,
    Debug,
    Deserialize,
    Eq,
    FromRedisValue,
    Identifiable,
    PartialEq,
    Queryable,
    Serialize,
    ToRedisArgs,
)]
#[diesel(table_name = events)]
#[diesel(belongs_to(User, foreign_key = created_by))]
#[diesel(belongs_to(Room, foreign_key = room))]
#[to_redis_args(serde)]
#[from_redis_value(serde)]
pub struct Event {
    pub id: EventId,
    pub id_serial: EventSerialId,
    pub title: EventTitle,
    pub description: EventDescription,
    pub room: RoomId,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_by: UserId,
    pub updated_at: DateTime<Utc>,
    pub is_all_day: Option<bool>,
    /// start datetime of the event
    pub starts_at: Option<DateTime<Utc>>,
    /// timezone of the start-datetime of the event
    pub starts_at_tz: Option<TimeZone>,
    /// end datetime of the event
    ///
    /// For recurring events contains the timestamp of the last occurrence
    pub ends_at: Option<DateTime<Utc>>,
    /// timezone of the ends_at datetime
    pub ends_at_tz: Option<TimeZone>,
    /// Only for recurring events, since ends_at contains the information
    /// about the last occurrence of the recurring series this duration value.
    ///
    /// MUST be used to calculate the event instances length.
    pub duration_secs: Option<i32>,
    pub recurrence_pattern: Option<String>,
    pub is_adhoc: bool,
    pub tenant_id: TenantId,
    pub revision: i32,
    pub show_meeting_details: bool,
}

impl From<Event> for inventory::Event {
    fn from(
        Event {
            id,
            id_serial,
            title,
            description,
            room,
            created_by,
            created_at,
            updated_by,
            updated_at,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
            duration_secs,
            recurrence_pattern,
            is_adhoc,
            tenant_id,
            revision,
            show_meeting_details,
        }: Event,
    ) -> Self {
        Self {
            id,
            id_serial: id_serial.into(),
            title,
            description,
            room,
            created_by,
            created_at: created_at.into(),
            updated_by,
            updated_at: updated_at.into(),
            is_adhoc,
            tenant_id,
            revision,
            show_meeting_details,
            date: (|| {
                Some(inventory::EventDate {
                    is_all_day: is_all_day?,
                    starts_at: starts_at?.into(),
                    starts_at_tz: starts_at_tz?,
                    ends_at: ends_at?.into(),
                    ends_at_tz: ends_at_tz?,
                    recurrence: recurrence_pattern.zip(duration_secs).map(
                        |(recurrence_pattern, duration_secs)| inventory::EventRecurrence {
                            recurrence_pattern,
                            duration_secs,
                        },
                    ),
                })
            })(),
        }
    }
}

impl From<&Event> for inventory::Event {
    fn from(value: &Event) -> Self {
        Self::from(value.clone())
    }
}

impl From<inventory::Event> for Event {
    fn from(event: inventory::Event) -> Self {
        Self {
            id: event.id,
            id_serial: event.id_serial.into(),
            room: event.room,
            created_by: event.created_by,
            created_at: event.created_at.into(),
            updated_by: event.updated_by,
            updated_at: event.updated_at.into(),
            is_all_day: event.is_all_day(),
            starts_at: event.starts_at().map(Into::into),
            starts_at_tz: event.starts_at_tz(),
            ends_at: event.ends_at().map(Into::into),
            ends_at_tz: event.ends_at_tz(),
            duration_secs: event.duration_secs(),
            recurrence_pattern: event.recurrence_pattern().map(ToString::to_string),
            is_adhoc: event.is_adhoc,
            tenant_id: event.tenant_id,
            revision: event.revision,
            show_meeting_details: event.show_meeting_details,
            // Note: Title and descrioption are note Copy, event partially moves
            // after here.
            title: event.title,
            description: event.description,
        }
    }
}

impl From<&inventory::Event> for Event {
    fn from(value: &inventory::Event) -> Self {
        Self::from(value.clone())
    }
}

impl Event {
    /// Returns the ends_at value of the first occurrence of the event
    pub fn ends_at_of_first_occurrence(&self) -> Option<(DateTime<Utc>, TimeZone)> {
        if self.recurrence_pattern.is_some() {
            // Recurring events have the last occurrence of the recurrence saved in the ends_at fields
            // So we get the starts_at_dt and add the duration_secs field to it
            if let (Some(starts_at_dt), Some(dur), Some(tz)) =
                (self.starts_at, self.duration_secs, self.ends_at_tz)
            {
                Some((starts_at_dt + chrono::Duration::seconds(i64::from(dur)), tz))
            } else {
                None
            }
        } else if let (Some(dt), Some(tz)) = (self.ends_at, self.ends_at_tz) {
            // Non recurring events just directly use the ends_at field from the db
            Some((dt, tz))
        } else {
            None
        }
    }
}

impl Event {
    #[tracing::instrument(err, skip_all)]
    pub async fn get(conn: &mut DbConnection, event_id: EventId) -> Result<Event> {
        let query = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::all_columns)
            .filter(events::id.eq(event_id))
            .filter(users::disabled_since.is_null());

        let event = query.first(conn).await?;

        Ok(event)
    }

    pub async fn get_all_with_creator(conn: &mut DbConnection) -> Result<Vec<(EventId, UserId)>> {
        let events = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select((events::id, events::created_by))
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await?;

        Ok(events)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_that_ended_before_including_rooms(
        conn: &mut DbConnection,
        date: DateTime<Utc>,
    ) -> Result<Vec<(EventId, RoomId)>> {
        events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select((events::id, events::room))
            .filter(events::ends_at.le(date))
            .filter(events::recurrence_pattern.is_null())
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_adhoc_created_before_including_rooms(
        conn: &mut DbConnection,
        date: DateTime<Utc>,
    ) -> Result<Vec<(EventId, RoomId)>> {
        events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select((events::id, events::room))
            .filter(events::created_at.le(date))
            .filter(events::is_adhoc.eq(true))
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_creator_including_rooms(
        conn: &mut DbConnection,
        created_by: UserId,
    ) -> Result<Vec<(EventId, RoomId)>> {
        events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select((events::id, events::room))
            .filter(events::created_by.eq(created_by))
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_finite_recurring(conn: &mut DbConnection) -> Result<Vec<Event>> {
        events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::all_columns)
            .filter(
                events::recurrence_pattern
                    .ilike("%UNTIL%")
                    .or(events::recurrence_pattern.ilike("%COUNT%")),
            )
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_updated_by_user(
        conn: &mut DbConnection,
        updated_by: UserId,
    ) -> Result<Vec<Event>> {
        events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::all_columns)
            .filter(events::updated_by.eq(updated_by))
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await
            .map_err(Into::into)
    }

    pub async fn get_all_with_invitee(
        conn: &mut DbConnection,
    ) -> Result<Vec<(EventId, RoomId, UserId)>> {
        let events = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .inner_join(event_invites::table.on(event_invites::event_id.eq(events::id)))
            .select((events::id, events::room, event_invites::invitee))
            .filter(users::disabled_since.is_null())
            .load(conn)
            .await?;

        Ok(events)
    }

    #[tracing::instrument(err, skip_all)]
    #[allow(clippy::type_complexity)]
    pub async fn get_with_related_items(
        conn: &mut DbConnection,
        user_id: UserId,
        event_id: EventId,
    ) -> Result<(
        Event,
        Option<EventInvite>,
        Room,
        Option<SipConfig>,
        bool,
        Option<EventSharedFolder>,
        Tariff,
        Option<EventTrainingParticipationReportParameterSet>,
    )> {
        let query =
            events::table
                .left_join(
                    event_invites::table.on(event_invites::event_id
                        .eq(events::id)
                        .and(event_invites::invitee.eq(user_id))),
                )
                .left_join(
                    event_favorites::table.on(event_favorites::event_id
                        .eq(events::id)
                        .and(event_favorites::user_id.eq(user_id))),
                )
                .left_join(
                    event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)),
                )
                .inner_join(rooms::table.on(events::room.eq(rooms::id)))
                .left_join(sip_configs::table.on(rooms::id.eq(sip_configs::room)))
                .inner_join(users::table.on(users::id.eq(events::created_by)))
                .inner_join(tariffs::table.on(tariffs::id.eq(users::tariff_id)))
                .left_join(
                    event_training_participation_report_parameter_sets::table
                        .on(event_training_participation_report_parameter_sets::event_id
                            .eq(events::id)),
                )
                .select((
                    events::all_columns,
                    event_invites::all_columns.nullable(),
                    rooms::all_columns,
                    sip_configs::all_columns.nullable(),
                    event_favorites::user_id.nullable().is_not_null(),
                    event_shared_folders::all_columns.nullable(),
                    tariffs::all_columns,
                    event_training_participation_report_parameter_sets::all_columns.nullable(),
                ))
                .filter(users::disabled_since.is_null())
                .filter(events::id.eq(event_id));
        Ok(query.first(conn).await?)
    }

    #[tracing::instrument(err, skip_all)]
    #[allow(clippy::type_complexity)]
    pub async fn get_with_room(
        conn: &mut DbConnection,
        event_id: EventId,
    ) -> Result<(Event, Room, Option<SipConfig>)> {
        let query = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .inner_join(rooms::table.on(events::room.eq(rooms::id)))
            .left_join(sip_configs::table.on(rooms::id.eq(sip_configs::room)))
            .select((
                events::all_columns,
                rooms::all_columns,
                sip_configs::all_columns.nullable(),
            ))
            .filter(events::id.eq(event_id))
            .filter(users::disabled_since.is_null());

        let (event, room, sip_config) = query.first(conn).await?;

        Ok((event, room, sip_config))
    }

    #[tracing::instrument(err, skip_all)]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub async fn get_all_for_user_paginated_as_stream(
        conn: &mut DbConnection,
        user: User,
        only_favorites: bool,
        invite_status_filter: Vec<EventInviteStatus>,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        created_after: Option<DateTime<Utc>>,
        adhoc: Option<bool>,
        time_independent: Option<bool>,
        cursor: Option<GetEventsCursor>,
    ) -> Result<
        impl Stream<
            Item = Result<(
                Event,
                Option<EventInvite>,
                Room,
                Option<SipConfig>,
                bool,
                Option<EventSharedFolder>,
                Tariff,
            )>,
        >,
    > {
        // Validate that the event is either created by the given user or an invite to the event
        // exists for the user
        let event_related_to_user_id = events::created_by
            .eq(user.id)
            .or(event_invites::invitee.eq(user.id));

        // Create query which select events and joins into the room of the event
        let mut query = events::table
            .left_join(
                event_invites::table.on(event_invites::event_id
                    .eq(events::id)
                    .and(event_invites::invitee.eq(user.id))),
            )
            .left_join(
                event_favorites::table.on(event_favorites::event_id
                    .eq(events::id)
                    .and(event_favorites::user_id.eq(user.id))),
            )
            .left_join(
                event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)),
            )
            .inner_join(rooms::table)
            .left_join(sip_configs::table.on(rooms::id.eq(sip_configs::room)))
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .inner_join(tariffs::table.on(tariffs::id.eq(users::tariff_id)))
            .select((
                events::all_columns,
                event_invites::all_columns.nullable(),
                rooms::all_columns,
                sip_configs::all_columns.nullable(),
                event_favorites::user_id.nullable().is_not_null(),
                event_shared_folders::all_columns.nullable(),
                tariffs::all_columns,
            ))
            .filter(events::tenant_id.eq(user.tenant_id))
            .filter(event_related_to_user_id)
            .filter(users::disabled_since.is_null())
            .order_by(events::starts_at.nullable().asc().nulls_first())
            .then_order_by(events::created_at.asc())
            .then_order_by(events::id.asc())
            .into_boxed::<Pg>();

        // Consider the start position as specified by the cursor
        if let Some(cursor) = cursor {
            if let Some(from_starts_at) = cursor.from_starts_at {
                let expr =
                    AsExpression::<Record<(Nullable<Timestamptz>,Timestamptz, Uuid)>>::as_expression((
                        events::starts_at,
                        events::created_at,
                        events::id
                    ));

                // Get all records that are behind the cursor position.
                // Records with no start date are considered to be less than the cursor specifies because
                // they don't pass the '>' comparison below (a comparison with NULL is always NULL).
                query =
                    query.filter(expr.gt((from_starts_at, cursor.from_created_at, cursor.from_id)));
            } else {
                let expr = AsExpression::<Record<(Timestamptz, Uuid)>>::as_expression((
                    events::created_at,
                    events::id,
                ));

                // Get all records that are behind the cursor position.
                // Records with a start date are considered to be greater than the cursor
                // specifies (which has no start date here).
                // For records without a start date the decision is based on the remaining values.
                query = query.filter(
                    events::starts_at.is_not_null().or(events::starts_at
                        .is_null()
                        .and(expr.gt((cursor.from_created_at, cursor.from_id)))),
                );
            }
        }

        // Add filters to query depending on the time_(min/max) parameters
        match (time_min, time_max) {
            (Some(time_min), Some(time_max)) => {
                // we have an overlap if any of these conditions matches:
                // - starts_at is between time_min and time_max
                // - ends_at is between time_min and time_max
                // - time_min is between starts_at and ends_at
                // - time_max is between starts_at and ends_at
                query = query.filter(
                    events::starts_at
                        .between(time_min, time_max)
                        .or(events::ends_at.between(time_min, time_max))
                        .or(time_min
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at))
                        .or(time_max
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at)),
                );
            }
            (Some(time_min), None) => {
                query = query.filter(events::ends_at.ge(time_min));
            }
            (None, Some(time_max)) => {
                query = query.filter(events::starts_at.le(time_max));
            }
            (None, None) => {
                // no filters to apply
            }
        }

        if let Some(created_before) = created_before {
            query = query.filter(events::created_at.le(created_before));
        }

        if let Some(created_after) = created_after {
            query = query.filter(events::created_at.ge(created_after));
        }

        if only_favorites {
            query = query.filter(event_favorites::user_id.is_not_null());
        }

        if let Some(is_adhoc) = adhoc {
            query = query.filter(events::is_adhoc.eq(is_adhoc));
        }

        if let Some(is_time_independent) = time_independent {
            if is_time_independent {
                query = query.filter(events::starts_at.is_null());
            } else {
                query = query.filter(events::starts_at.is_not_null());
            }
        }

        if !invite_status_filter.is_empty() {
            if invite_status_filter.contains(&EventInviteStatus::Accepted) {
                // edge case to allow event creators to filter created events by 'accepted'
                query = query.filter(
                    event_invites::status
                        .eq_any(invite_status_filter)
                        .or(event_invites::status.is_null()),
                );
            } else {
                query = query.filter(event_invites::status.eq_any(invite_status_filter));
            }
        }

        let stream = query
            .load_stream::<(
                Event,
                Option<EventInvite>,
                Room,
                Option<SipConfig>,
                bool,
                Option<EventSharedFolder>,
                Tariff,
            )>(conn)
            .await?;

        Ok(convert_diesel_query_results(stream))
    }

    #[tracing::instrument(err, skip_all)]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub async fn get_all_exceptions_for_user_paginated_as_stream(
        conn: &mut DbConnection,
        user: User,
        only_favorites: bool,
        invite_status_filter: Vec<EventInviteStatus>,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        created_after: Option<DateTime<Utc>>,
        adhoc: Option<bool>,
        time_independent: Option<bool>,
        cursor: Option<GetEventExceptionsCursor>,
    ) -> Result<impl Stream<Item = Result<(EventException, Event)>>> {
        // Validate that the event is either created by the given user or an invite to the event
        // exists for the user
        let event_related_to_user_id = events::created_by
            .eq(user.id)
            .or(event_invites::invitee.eq(user.id));

        let mut query = event_exceptions::table
            .inner_join(events::table.on(event_exceptions::event_id.eq(events::id)))
            .left_join(
                event_invites::table.on(event_invites::event_id
                    .eq(events::id)
                    .and(event_invites::invitee.eq(user.id))),
            )
            .left_join(
                event_favorites::table.on(event_favorites::event_id
                    .eq(events::id)
                    .and(event_favorites::user_id.eq(user.id))),
            )
            .left_join(
                event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)),
            )
            .inner_join(rooms::table.on(events::room.eq(rooms::id)))
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select((event_exceptions::all_columns, events::all_columns))
            .filter(events::tenant_id.eq(user.tenant_id))
            .filter(event_related_to_user_id)
            .filter(users::disabled_since.is_null())
            .order_by(event_exceptions::starts_at.nullable().asc().nulls_first())
            .then_order_by(event_exceptions::created_at.asc())
            .then_order_by(event_exceptions::event_id.asc())
            .then_order_by(event_exceptions::exception_date.asc())
            .into_boxed::<Pg>();

        // Consider the start position as specified by the cursor
        if let Some(cursor) = cursor {
            if let Some(from_starts_at) = cursor.from_starts_at {
                let expr = AsExpression::<
                    Record<(Nullable<Timestamptz>, Timestamptz, Uuid, Timestamptz)>,
                >::as_expression((
                    event_exceptions::starts_at,
                    event_exceptions::created_at,
                    event_exceptions::event_id,
                    event_exceptions::exception_date,
                ));

                // Get all records that are behind the cursor position.
                // Records with no start date are considered to be less than the cursor specifies because
                // they don't pass the '>' comparison below (a comparison with NULL is always NULL).
                query = query.filter(expr.gt((
                    from_starts_at,
                    cursor.from_created_at,
                    cursor.from_id,
                    cursor.from_exception_date,
                )));
            } else {
                let expr =
                    AsExpression::<Record<(Timestamptz, Uuid, Timestamptz)>>::as_expression((
                        event_exceptions::created_at,
                        event_exceptions::event_id,
                        event_exceptions::exception_date,
                    ));

                // Get all records that are behind the cursor position.
                // Records with a start date are considered to be greater than the cursor
                // specifies (which has no start date here).
                // For records without a start date the decision is based on the remaining values.
                query = query.filter(event_exceptions::starts_at.is_not_null().or(
                    event_exceptions::starts_at.is_null().and(expr.gt((
                        cursor.from_created_at,
                        cursor.from_id,
                        cursor.from_exception_date,
                    ))),
                ));
            }
        }

        // Add filters to query depending on the time_(min/max) parameters
        match (time_min, time_max) {
            (Some(time_min), Some(time_max)) => {
                // we have an overlap if any of these conditions matches:
                // - starts_at is between time_min and time_max
                // - ends_at is between time_min and time_max
                // - time_min is between starts_at and ends_at
                // - time_max is between starts_at and ends_at
                query = query.filter(
                    events::starts_at
                        .between(time_min, time_max)
                        .or(events::ends_at.between(time_min, time_max))
                        .or(time_min
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at))
                        .or(time_max
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at)),
                );
            }
            (Some(time_min), None) => {
                query = query.filter(events::ends_at.ge(time_min));
            }
            (None, Some(time_max)) => {
                query = query.filter(events::starts_at.le(time_max));
            }
            (None, None) => {
                // no filters to apply
            }
        }

        if let Some(created_before) = created_before {
            query = query.filter(events::created_at.le(created_before));
        }

        if let Some(created_after) = created_after {
            query = query.filter(events::created_at.ge(created_after));
        }

        if only_favorites {
            query = query.filter(event_favorites::user_id.is_not_null());
        }

        if let Some(is_adhoc) = adhoc {
            query = query.filter(events::is_adhoc.eq(is_adhoc));
        }

        if let Some(is_time_independent) = time_independent {
            if is_time_independent {
                query = query.filter(events::starts_at.is_null());
            } else {
                query = query.filter(events::starts_at.is_not_null());
            }
        }

        if !invite_status_filter.is_empty() {
            if invite_status_filter.contains(&EventInviteStatus::Accepted) {
                // edge case to allow event creators to filter created events by 'accepted'
                query = query.filter(
                    event_invites::status
                        .eq_any(invite_status_filter)
                        .or(event_invites::status.is_null()),
                );
            } else {
                query = query.filter(event_invites::status.eq_any(invite_status_filter));
            }
        }

        let stream = query.load_stream::<(EventException, Event)>(conn).await?;

        Ok(convert_diesel_query_results(stream))
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_user(
        conn: &mut DbConnection,
        user: User,
        only_recurring: bool,
    ) -> Result<Vec<Event>> {
        // Filter applied to all events which validates that the event is either created by
        // the given user or an invite to the event exists for the user
        let event_related_to_user_id = events::created_by
            .eq(user.id)
            .or(event_invites::invitee.eq(user.id));

        // Create query which select events and joins into the room of the event
        let mut query = events::table
            .left_join(
                event_invites::table.on(event_invites::event_id
                    .eq(events::id)
                    .and(event_invites::invitee.eq(user.id))),
            )
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::all_columns)
            .filter(events::tenant_id.eq(user.tenant_id))
            .filter(event_related_to_user_id)
            .filter(users::disabled_since.is_null())
            .order_by(events::starts_at.nullable().asc().nulls_first())
            .then_order_by(events::created_at.asc())
            .then_order_by(events::id.asc())
            .into_boxed::<Pg>();

        if only_recurring {
            query = query.filter(events::recurrence_pattern.is_not_null());
        }

        let events: Vec<Event> = query.load(conn).await?;

        Ok(events)
    }

    #[tracing::instrument(err, skip_all)]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub async fn get_all_for_user_paginated(
        conn: &mut DbConnection,
        user: User,
        only_favorites: bool,
        invite_status_filter: Vec<EventInviteStatus>,
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        created_after: Option<DateTime<Utc>>,
        adhoc: Option<bool>,
        time_independent: Option<bool>,
        cursor: Option<GetEventsCursor>,
        limit: i64,
    ) -> Result<
        Vec<(
            Event,
            Option<EventInvite>,
            Room,
            Option<SipConfig>,
            Vec<EventException>,
            bool,
            Option<EventSharedFolder>,
            Tariff,
            Option<TrainingParticipationReportParameterSet>,
        )>,
    > {
        // Filter applied to all events which validates that the event is either created by
        // the given user or an invite to the event exists for the user
        let event_related_to_user_id = events::created_by
            .eq(user.id)
            .or(event_invites::invitee.eq(user.id));

        // Create query which select events and joins into the room of the event
        let mut query = events::table
            .left_join(
                event_invites::table.on(event_invites::event_id
                    .eq(events::id)
                    .and(event_invites::invitee.eq(user.id))),
            )
            .left_join(
                event_favorites::table.on(event_favorites::event_id
                    .eq(events::id)
                    .and(event_favorites::user_id.eq(user.id))),
            )
            .left_join(
                event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)),
            )
            .inner_join(rooms::table)
            .left_join(sip_configs::table.on(rooms::id.eq(sip_configs::room)))
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .inner_join(tariffs::table.on(tariffs::id.eq(users::tariff_id)))
            .select((
                events::all_columns,
                event_invites::all_columns.nullable(),
                rooms::all_columns,
                sip_configs::all_columns.nullable(),
                event_favorites::user_id.nullable().is_not_null(),
                event_shared_folders::all_columns.nullable(),
                tariffs::all_columns,
            ))
            .filter(events::tenant_id.eq(user.tenant_id))
            .filter(event_related_to_user_id)
            .filter(users::disabled_since.is_null())
            .order_by(events::starts_at.nullable().asc().nulls_first())
            .then_order_by(events::created_at.asc())
            .then_order_by(events::id)
            .limit(limit)
            .into_boxed::<Pg>();

        // Tuples/Composite types are ordered by lexical ordering
        if let Some(cursor) = cursor {
            if let Some(from_starts_at) = cursor.from_starts_at {
                let expr =
                    AsExpression::<Record<(Nullable<Timestamptz>,Timestamptz, Uuid)>>::as_expression((
                        events::starts_at,
                        events::created_at,
                        events::id
                    ));

                query =
                    query.filter(expr.gt((from_starts_at, cursor.from_created_at, cursor.from_id)));
            } else {
                // TODO: This doesn't work for records with a start date (compare to get_all_[exceptions_]for_user_paginated_as_stream).
                let expr = AsExpression::<Record<(Timestamptz, Uuid)>>::as_expression((
                    events::created_at,
                    events::id,
                ));

                query = query.filter(expr.gt((cursor.from_created_at, cursor.from_id)));
            }
        }

        // Add filters to query depending on the time_(min/max) parameters
        match (time_min, time_max) {
            (Some(time_min), Some(time_max)) => {
                // we have an overlap if any of these conditions matches:
                // - starts_at is between time_min and time_max
                // - ends_at is between time_min and time_max
                // - time_min is between starts_at and ends_at
                // - time_max is between starts_at and ends_at
                query = query.filter(
                    events::starts_at
                        .between(time_min, time_max)
                        .or(events::ends_at.between(time_min, time_max))
                        .or(time_min
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at))
                        .or(time_max
                            .into_sql::<Nullable<Timestamptz>>()
                            .between(events::starts_at, events::ends_at)),
                );
            }
            (Some(time_min), None) => {
                query = query.filter(events::ends_at.ge(time_min));
            }
            (None, Some(time_max)) => {
                query = query.filter(events::starts_at.le(time_max));
            }
            (None, None) => {
                // no filters to apply
            }
        }

        if let Some(created_before) = created_before {
            query = query.filter(events::created_at.le(created_before));
        }

        if let Some(created_after) = created_after {
            query = query.filter(events::created_at.ge(created_after));
        }

        if only_favorites {
            query = query.filter(event_favorites::user_id.is_not_null());
        }

        if let Some(is_adhoc) = adhoc {
            query = query.filter(events::is_adhoc.eq(is_adhoc));
        }

        if let Some(is_time_independent) = time_independent {
            if is_time_independent {
                query = query.filter(events::starts_at.is_null());
            } else {
                query = query.filter(events::starts_at.is_not_null());
            }
        }

        if !invite_status_filter.is_empty() {
            if invite_status_filter.contains(&EventInviteStatus::Accepted) {
                // edge case to allow event creators to filter created events by 'accepted'
                query = query.filter(
                    event_invites::status
                        .eq_any(invite_status_filter)
                        .or(event_invites::status.is_null()),
                );
            } else {
                query = query.filter(event_invites::status.eq_any(invite_status_filter));
            }
        }

        let events_with_invite_and_room: Vec<(
            Event,
            Option<EventInvite>,
            Room,
            Option<SipConfig>,
            bool,
            Option<EventSharedFolder>,
            Tariff,
        )> = query.load(conn).await?;

        let mut events_with_invite_room_and_exceptions =
            Vec::with_capacity(events_with_invite_and_room.len());

        for (event, invite, room, sip_config, is_favorite, shared_folders, tariff) in
            events_with_invite_and_room
        {
            let exceptions = if event.recurrence_pattern.is_some() {
                event_exceptions::table
                    .filter(event_exceptions::event_id.eq(event.id))
                    .load(conn)
                    .await?
            } else {
                vec![]
            };
            // TODO: remove this
            let training_participation_report_parameter_set = None;

            events_with_invite_room_and_exceptions.push((
                event,
                invite,
                room,
                sip_config,
                exceptions,
                is_favorite,
                shared_folders,
                tariff,
                training_participation_report_parameter_set,
            ));
        }

        Ok(events_with_invite_room_and_exceptions)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_by_id(conn: &mut DbConnection, event_id: EventId) -> Result<()> {
        diesel::delete(events::table)
            .filter(events::id.eq(event_id))
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Returns the [`Event`] in the given [`RoomId`].
    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_room(conn: &mut DbConnection, room_id: RoomId) -> Result<Option<Event>> {
        let event = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::all_columns)
            .filter(events::room.eq(room_id))
            .filter(users::disabled_since.is_null())
            .first::<Event>(conn)
            .await
            .optional()?;
        Ok(event)
    }

    /// Returns a [`EventId`] for the given [`RoomId`].
    #[tracing::instrument(err, skip_all)]
    pub async fn get_id_for_room(
        conn: &mut DbConnection,
        room_id: RoomId,
    ) -> Result<Option<EventId>> {
        let query = events::table
            .inner_join(users::table.on(users::id.eq(events::created_by)))
            .select(events::id)
            .filter(events::room.eq(room_id))
            .filter(users::disabled_since.is_null());

        let events = query.first(conn).await.optional()?;

        Ok(events)
    }

    /// Deletes all [`Event`]s in a given [`RoomId`]
    ///
    /// Fastpath for deleting multiple events in room
    #[tracing::instrument(err, skip_all)]
    pub async fn delete_for_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
        diesel::delete(events::table)
            .filter(events::room.eq(room_id))
            .execute(conn)
            .await?;

        Ok(())
    }
}
