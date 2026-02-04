// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use derive_more::{AsRef, Display, From, FromStr, Into};
use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl, NullableExpressionMethods,
    OptionalExtension, PgSortExpressionMethods, QueryDsl, Queryable,
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    sql_types::{Nullable, Record, Timestamptz, Uuid},
};
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use futures_core::Stream;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_diesel_newtype::DieselNewtype;
use opentalk_types_common::{
    events::{
        EventDescription, EventId, EventTitle,
        invites::{EventInviteStatus, InviteRole},
    },
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    sql_enum,
    tenants::TenantId,
    time::TimeZone,
    training_participation_report::{TimeRange, TrainingParticipationReportParameterSet},
    users::UserId,
};
use redis_args::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

use self::shared_folders::EventSharedFolder;
use crate::{
    newtypes::Duration,
    paginate::Paginate as _,
    rooms::Room,
    schema::{
        event_exceptions, event_favorites, event_invites, event_shared_folders,
        event_training_participation_report_parameter_sets, events, rooms, sip_configs, tariffs,
        users,
    },
    sip_configs::SipConfig,
    tariffs::Tariff,
    users::User,
    utils::convert_diesel_query_results,
};

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
#[diesel(sql_type = diesel::sql_types::BigInt)]
pub struct EventSerialId(i64);

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
pub struct EventExceptionId(uuid::Uuid);

impl From<opentalk_inventory::EventExceptionId> for EventExceptionId {
    fn from(value: opentalk_inventory::EventExceptionId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}

impl From<EventExceptionId> for opentalk_inventory::EventExceptionId {
    fn from(EventExceptionId(id): EventExceptionId) -> Self {
        Self::from(id)
    }
}

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

impl From<opentalk_inventory::EventInviteId> for EventInviteId {
    fn from(value: opentalk_inventory::EventInviteId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}

impl From<EventInviteId> for opentalk_inventory::EventInviteId {
    fn from(value: EventInviteId) -> Self {
        Self::from(uuid::Uuid::from(value))
    }
}

pub mod email_invites;
pub mod shared_folders;

#[derive(
    Debug,
    Clone,
    Queryable,
    Identifiable,
    Associations,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ToRedisArgs,
    FromRedisValue,
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
    pub is_time_independent: bool,
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

impl From<Event> for opentalk_inventory::Event {
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
            is_time_independent: _,
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
                Some(opentalk_inventory::EventDate {
                    is_all_day: is_all_day?,
                    starts_at: starts_at?.into(),
                    starts_at_tz: starts_at_tz?,
                    ends_at: ends_at?.into(),
                    ends_at_tz: ends_at_tz?,
                    recurrence: recurrence_pattern.zip(duration_secs).map(
                        |(recurrence_pattern, duration_secs)| opentalk_inventory::EventRecurrence {
                            recurrence_pattern,
                            duration_secs,
                        },
                    ),
                })
            })(),
        }
    }
}

impl From<&Event> for opentalk_inventory::Event {
    fn from(value: &Event) -> Self {
        Self::from(value.clone())
    }
}

impl From<opentalk_inventory::Event> for Event {
    fn from(event: opentalk_inventory::Event) -> Self {
        Self {
            id: event.id,
            id_serial: event.id_serial.into(),
            room: event.room,
            created_by: event.created_by,
            created_at: event.created_at.into(),
            updated_by: event.updated_by,
            updated_at: event.updated_at.into(),
            is_time_independent: event.is_time_independent(),
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

impl From<&opentalk_inventory::Event> for Event {
    fn from(value: &opentalk_inventory::Event) -> Self {
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

pub struct GetEventsCursor {
    pub from_id: EventId,
    pub from_created_at: DateTime<Utc>,
    pub from_starts_at: Option<DateTime<Utc>>,
}

impl From<GetEventsCursor> for opentalk_inventory::GetEventsCursor {
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

impl From<opentalk_inventory::GetEventsCursor> for GetEventsCursor {
    fn from(value: opentalk_inventory::GetEventsCursor) -> Self {
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

impl From<GetEventExceptionsCursor> for opentalk_inventory::GetEventExceptionsCursor {
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

impl From<opentalk_inventory::GetEventExceptionsCursor> for GetEventExceptionsCursor {
    fn from(value: opentalk_inventory::GetEventExceptionsCursor) -> Self {
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
            query = query.filter(events::is_time_independent.eq(is_time_independent));
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
            query = query.filter(events::is_time_independent.eq(is_time_independent));
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
            query = query.filter(events::is_time_independent.eq(is_time_independent));
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

#[derive(Debug, Insertable)]
#[diesel(table_name = events)]
pub struct NewEvent {
    pub title: EventTitle,
    pub description: EventDescription,
    pub room: RoomId,
    pub created_by: UserId,
    pub updated_by: UserId,
    pub is_time_independent: bool,
    pub is_all_day: Option<bool>,
    pub starts_at: Option<DateTime<Tz>>,
    pub starts_at_tz: Option<TimeZone>,
    pub ends_at: Option<DateTime<Tz>>,
    pub ends_at_tz: Option<TimeZone>,
    pub duration_secs: Option<i32>,
    pub recurrence_pattern: Option<String>,
    pub is_adhoc: bool,
    pub tenant_id: TenantId,
    pub show_meeting_details: bool,
}

impl From<opentalk_inventory::NewEvent> for NewEvent {
    fn from(new_event: opentalk_inventory::NewEvent) -> Self {
        Self {
            room: new_event.room,
            created_by: new_event.created_by,
            updated_by: new_event.updated_by,
            is_time_independent: new_event.is_time_independent(),
            is_all_day: new_event.is_all_day(),
            starts_at: new_event.starts_at(),
            starts_at_tz: new_event.starts_at_tz(),
            ends_at: new_event.ends_at(),
            ends_at_tz: new_event.ends_at_tz(),
            duration_secs: new_event.duration_secs(),
            recurrence_pattern: new_event.recurrence_pattern().map(ToString::to_string),
            // Note: Title and description are not Copy, value partially moves
            // after here.
            title: new_event.title,
            description: new_event.description,
            is_adhoc: new_event.is_adhoc,
            tenant_id: new_event.tenant_id,
            show_meeting_details: new_event.show_meeting_details,
        }
    }
}

impl NewEvent {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<Event> {
        let query = self.insert_into(events::table);

        let event = query.get_result(conn).await?;

        Ok(event)
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = events)]
pub struct UpdateEvent {
    pub title: Option<EventTitle>,
    pub description: Option<EventDescription>,
    pub updated_by: UserId,
    pub updated_at: DateTime<Utc>,
    pub is_time_independent: Option<bool>,
    pub is_all_day: Option<Option<bool>>,
    pub starts_at: Option<Option<DateTime<Tz>>>,
    pub starts_at_tz: Option<Option<TimeZone>>,
    pub ends_at: Option<Option<DateTime<Tz>>>,
    pub ends_at_tz: Option<Option<TimeZone>>,
    pub duration_secs: Option<Option<i32>>,
    pub recurrence_pattern: Option<Option<String>>,
    pub is_adhoc: Option<bool>,
    pub show_meeting_details: Option<bool>,
}

impl From<opentalk_inventory::UpdateEvent> for UpdateEvent {
    fn from(update_event: opentalk_inventory::UpdateEvent) -> Self {
        Self {
            updated_by: update_event.updated_by,
            updated_at: update_event.updated_at.into(),
            is_time_independent: Some(update_event.is_time_independent()),
            is_all_day: Some(update_event.is_all_day()),
            starts_at: Some(update_event.starts_at()),
            starts_at_tz: Some(update_event.starts_at_tz()),
            ends_at: Some(update_event.ends_at()),
            ends_at_tz: Some(update_event.ends_at_tz()),
            duration_secs: Some(update_event.duration_secs()),
            recurrence_pattern: Some(update_event.recurrence_pattern().map(ToString::to_string)),
            is_adhoc: update_event.is_adhoc,
            show_meeting_details: update_event.show_meeting_details,
            title: update_event.title,
            description: update_event.description,
        }
    }
}

impl UpdateEvent {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(self, conn: &mut DbConnection, event_id: EventId) -> Result<Event> {
        let query = diesel::update(events::table)
            .filter(events::id.eq(event_id))
            .set((self, events::revision.eq(events::revision + 1)))
            .returning(events::all_columns);

        let event = query.get_result(conn).await?;

        Ok(event)
    }
}

sql_enum!(
    EventExceptionKind,
    "event_exception_kind",
    EventExceptionKindType,
    {
        Modified = b"modified",
        Cancelled = b"cancelled",
    }
);

impl From<EventExceptionKind> for opentalk_inventory::EventExceptionKind {
    fn from(value: EventExceptionKind) -> Self {
        match value {
            EventExceptionKind::Modified => Self::Modified,
            EventExceptionKind::Cancelled => Self::Cancelled,
        }
    }
}

impl From<opentalk_inventory::EventExceptionKind> for EventExceptionKind {
    fn from(value: opentalk_inventory::EventExceptionKind) -> Self {
        match value {
            opentalk_inventory::EventExceptionKind::Modified => Self::Modified,
            opentalk_inventory::EventExceptionKind::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Queryable, Identifiable, Associations)]
#[diesel(table_name = event_exceptions)]
#[diesel(belongs_to(Event, foreign_key = event_id))]
#[diesel(belongs_to(User, foreign_key = created_by))]
pub struct EventException {
    pub id: EventExceptionId,
    pub event_id: EventId,
    pub exception_date: DateTime<Utc>,
    pub exception_date_tz: TimeZone,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub kind: EventExceptionKind,
    pub title: Option<EventTitle>,
    pub description: Option<EventDescription>,
    pub is_all_day: Option<bool>,
    pub starts_at: Option<DateTime<Utc>>,
    pub starts_at_tz: Option<TimeZone>,
    pub ends_at: Option<DateTime<Utc>>,
    pub ends_at_tz: Option<TimeZone>,
}

impl From<EventException> for opentalk_inventory::EventException {
    fn from(
        EventException {
            id,
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            created_at,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: EventException,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            created_at: created_at.into(),
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at: starts_at.map(Into::into),
            starts_at_tz,
            ends_at: ends_at.map(Into::into),
            ends_at_tz,
        }
    }
}

impl From<opentalk_inventory::EventException> for EventException {
    fn from(
        opentalk_inventory::EventException {
            id,
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            created_at,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_inventory::EventException,
    ) -> Self {
        Self {
            id: id.into(),
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            created_at: created_at.into(),
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at: starts_at.map(Into::into),
            starts_at_tz,
            ends_at: ends_at.map(Into::into),
            ends_at_tz,
        }
    }
}

impl EventException {
    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_event(
        conn: &mut DbConnection,
        event_id: EventId,
        datetime: DateTime<Utc>,
    ) -> Result<Option<EventException>> {
        let query = event_exceptions::table.filter(
            event_exceptions::event_id
                .eq(event_id)
                .and(event_exceptions::exception_date.eq(datetime)),
        );

        let exceptions = query.first(conn).await.optional()?;

        Ok(exceptions)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_all_for_event(
        conn: &mut DbConnection,
        event_id: EventId,
        datetimes: &[&DateTime<Utc>],
    ) -> Result<Vec<EventException>> {
        let query = event_exceptions::table.filter(
            event_exceptions::event_id
                .eq(event_id)
                .and(event_exceptions::exception_date.eq_any(datetimes)),
        );

        let exceptions = query.load(conn).await.optional()?.unwrap_or_default();

        Ok(exceptions)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete_all_for_event(conn: &mut DbConnection, event_id: EventId) -> Result<()> {
        let query =
            diesel::delete(event_exceptions::table).filter(event_exceptions::event_id.eq(event_id));

        query.execute(conn).await?;

        Ok(())
    }
}

#[derive(Debug, Insertable)]
#[diesel(table_name = event_exceptions)]
pub struct NewEventException {
    pub event_id: EventId,
    pub exception_date: DateTime<Utc>,
    pub exception_date_tz: TimeZone,
    pub created_by: UserId,
    pub kind: EventExceptionKind,
    pub title: Option<EventTitle>,
    pub description: Option<EventDescription>,
    pub is_all_day: Option<bool>,
    pub starts_at: Option<DateTime<Tz>>,
    pub starts_at_tz: Option<TimeZone>,
    pub ends_at: Option<DateTime<Tz>>,
    pub ends_at_tz: Option<TimeZone>,
}

impl NewEventException {
    #[tracing::instrument(err, skip_all)]
    pub async fn insert(self, conn: &mut DbConnection) -> Result<EventException> {
        let query = self.insert_into(event_exceptions::table);

        let event_exception = query.get_result(conn).await?;

        Ok(event_exception)
    }
}

impl From<NewEventException> for opentalk_inventory::NewEventException {
    fn from(
        NewEventException {
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: NewEventException,
    ) -> Self {
        Self {
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }
    }
}

impl From<opentalk_inventory::NewEventException> for NewEventException {
    fn from(
        opentalk_inventory::NewEventException {
            event_id,
            exception_date,
            exception_date_tz,
            created_by,
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_inventory::NewEventException,
    ) -> Self {
        Self {
            event_id,
            exception_date: exception_date.into(),
            exception_date_tz,
            created_by,
            kind: kind.into(),
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = event_exceptions)]
pub struct UpdateEventException {
    pub kind: Option<EventExceptionKind>,
    pub title: Option<Option<EventTitle>>,
    pub description: Option<Option<EventDescription>>,
    pub is_all_day: Option<Option<bool>>,
    pub starts_at: Option<Option<DateTime<Tz>>>,
    pub starts_at_tz: Option<Option<TimeZone>>,
    pub ends_at: Option<Option<DateTime<Tz>>>,
    pub ends_at_tz: Option<Option<TimeZone>>,
}

impl From<opentalk_inventory::UpdateEventException> for UpdateEventException {
    fn from(
        opentalk_inventory::UpdateEventException {
            kind,
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }: opentalk_inventory::UpdateEventException,
    ) -> Self {
        Self {
            kind: kind.map(Into::into),
            title,
            description,
            is_all_day,
            starts_at,
            starts_at_tz,
            ends_at,
            ends_at_tz,
        }
    }
}

impl UpdateEventException {
    #[tracing::instrument(err, skip_all)]
    pub async fn apply(
        self,
        conn: &mut DbConnection,
        event_exception_id: EventExceptionId,
    ) -> Result<EventException> {
        let query = diesel::update(event_exceptions::table)
            .filter(event_exceptions::id.eq(event_exception_id))
            .set(self)
            .returning(event_exceptions::all_columns);

        let exception = query.get_result(conn).await?;

        Ok(exception)
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

impl From<EventInvite> for opentalk_inventory::EventInvite {
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

impl From<opentalk_inventory::EventInvite> for EventInvite {
    fn from(
        opentalk_inventory::EventInvite {
            id,
            event_id,
            invitee,
            created_by,
            created_at,
            status,
            role,
        }: opentalk_inventory::EventInvite,
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

impl From<opentalk_inventory::NewEventInvite> for NewEventInvite {
    fn from(
        opentalk_inventory::NewEventInvite {
            event_id,
            invitee,
            role,
            created_by,
            created_at,
        }: opentalk_inventory::NewEventInvite,
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

impl From<opentalk_inventory::UpdateEventInvite> for UpdateEventInvite {
    fn from(
        opentalk_inventory::UpdateEventInvite { status, role }: opentalk_inventory::UpdateEventInvite,
    ) -> Self {
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
    for opentalk_inventory::EventTrainingParticipationReportParameterSet
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

impl From<opentalk_inventory::EventTrainingParticipationReportParameterSet>
    for EventTrainingParticipationReportParameterSet
{
    fn from(
        opentalk_inventory::EventTrainingParticipationReportParameterSet {
            event_id,
            initial_checkpoint_delay,
            checkpoint_interval,
        }: opentalk_inventory::EventTrainingParticipationReportParameterSet,
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

impl From<opentalk_inventory::UpdateEventTrainingParticipationReportParameterSet>
    for UpdateEventTrainingParticipationReportParameterSet
{
    fn from(
        opentalk_inventory::UpdateEventTrainingParticipationReportParameterSet {
            initial_checkpoint_delay_after,
            initial_checkpoint_delay_within,
            checkpoint_interval_after,
            checkpoint_interval_within,
        }: opentalk_inventory::UpdateEventTrainingParticipationReportParameterSet,
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
