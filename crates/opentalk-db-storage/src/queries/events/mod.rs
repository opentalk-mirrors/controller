// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains events database queries

pub mod cursor;

mod email_invite;
mod exception;
mod favorite;
mod invite;
mod shared_folder;
mod training_participation_report;

use chrono::{DateTime, Utc};
use diesel::{
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    sql_types::{Nullable, Record, Timestamptz, Uuid},
};
use diesel_async::RunQueryDsl;
pub use email_invite::*;
pub use exception::*;
pub use favorite::*;
use futures_core::Stream;
pub use invite::*;
use opentalk_database::{DatabaseError, DbConnection, Result};
use opentalk_types_common::{
    events::{EventId, invites::EventInviteStatus},
    rooms::RoomId,
    training_participation_report::TrainingParticipationReportParameterSet,
    users::UserId,
};
pub use shared_folder::*;
pub use training_participation_report::*;

use crate::{
    queries::events::cursor::{GetEventExceptionsCursor, GetEventsCursor},
    schema::{
        event_exceptions, event_favorites, event_invites, event_shared_folders,
        event_training_participation_report_parameter_sets, events, rooms, sip_configs, tariffs,
        users,
    },
    tables::{
        event_exceptions::EventException,
        event_invites::EventInvite,
        event_shared_folders::EventSharedFolder,
        event_training_participation_report_parameter_sets::EventTrainingParticipationReportParameterSet,
        events::{Event, NewEvent, UpdateEvent},
        rooms::Room,
        sip_configs::SipConfig,
        tariffs::Tariff,
    },
    users::User,
    utils::convert_diesel_query_results,
};

#[tracing::instrument(err, skip_all)]
pub async fn get_event(conn: &mut DbConnection, event_id: EventId) -> Result<Event> {
    let query = events::table
        .inner_join(users::table.on(users::id.eq(events::created_by)))
        .select(events::all_columns)
        .filter(events::id.eq(event_id))
        .filter(users::disabled_since.is_null());

    let event = query.first(conn).await?;

    Ok(event)
}

pub async fn get_all_events_with_creator(
    conn: &mut DbConnection,
) -> Result<Vec<(EventId, UserId)>> {
    let events = events::table
        .inner_join(users::table.on(users::id.eq(events::created_by)))
        .select((events::id, events::created_by))
        .filter(users::disabled_since.is_null())
        .load(conn)
        .await?;

    Ok(events)
}

#[tracing::instrument(err, skip_all)]
pub async fn get_all_events_that_ended_before_including_rooms(
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
pub async fn get_all_events_adhoc_created_before_including_rooms(
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
pub async fn get_all_events_for_creator_including_rooms(
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
pub async fn get_all_events_finite_recurring(conn: &mut DbConnection) -> Result<Vec<Event>> {
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
pub async fn get_all_events_updated_by_user(
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

pub async fn get_all_events_with_invitee(
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
    let query = events::table
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
        .left_join(event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)))
        .inner_join(rooms::table.on(events::room.eq(rooms::id)))
        .left_join(sip_configs::table.on(rooms::id.eq(sip_configs::room)))
        .inner_join(users::table.on(users::id.eq(events::created_by)))
        .inner_join(tariffs::table.on(tariffs::id.eq(users::tariff_id)))
        .left_join(
            event_training_participation_report_parameter_sets::table
                .on(event_training_participation_report_parameter_sets::event_id.eq(events::id)),
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
pub async fn get_all_events_for_user_paginated_as_stream(
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
        .left_join(event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)))
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
                AsExpression::<Record<(Nullable<Timestamptz>, Timestamptz, Uuid)>>::as_expression(
                    (events::starts_at, events::created_at, events::id),
                );

            // Get all records that are behind the cursor position.
            // Records with no start date are considered to be less than the cursor specifies because
            // they don't pass the '>' comparison below (a comparison with NULL is always NULL).
            query = query.filter(expr.gt((from_starts_at, cursor.from_created_at, cursor.from_id)));
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
pub async fn get_all_events_exceptions_for_user_paginated_as_stream(
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
        .left_join(event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)))
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
            let expr = AsExpression::<Record<(Timestamptz, Uuid, Timestamptz)>>::as_expression((
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
pub async fn get_all_events_for_user(
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
pub async fn get_all_events_for_user_paginated(
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
        .left_join(event_shared_folders::table.on(event_shared_folders::event_id.eq(events::id)))
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
                AsExpression::<Record<(Nullable<Timestamptz>, Timestamptz, Uuid)>>::as_expression(
                    (events::starts_at, events::created_at, events::id),
                );

            query = query.filter(expr.gt((from_starts_at, cursor.from_created_at, cursor.from_id)));
        } else {
            // TODO: This doesn't work for records with a start date (compare to get_all_events_[exceptions_]for_user_paginated_as_stream).
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
pub async fn get_event_for_room(conn: &mut DbConnection, room_id: RoomId) -> Result<Option<Event>> {
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
pub async fn get_event_id_for_room(
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
pub async fn delete_event_for_room(conn: &mut DbConnection, room_id: RoomId) -> Result<()> {
    diesel::delete(events::table)
        .filter(events::room.eq(room_id))
        .execute(conn)
        .await?;

    Ok(())
}

/// Creates a new event.
#[tracing::instrument(err, skip_all)]
pub async fn create_event(conn: &mut DbConnection, new_event: NewEvent) -> Result<Event> {
    diesel::insert_into(events::table)
        .values(new_event)
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}

/// Update an event.
#[tracing::instrument(err, skip_all)]
pub async fn update_event(
    conn: &mut DbConnection,
    event_id: EventId,
    updated_event: UpdateEvent,
) -> Result<Event> {
    diesel::update(events::table)
        .filter(events::id.eq(event_id))
        .set((updated_event, events::revision.eq(events::revision + 1)))
        .get_result(conn)
        .await
        .map_err(DatabaseError::from)
}
