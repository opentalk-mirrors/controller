// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle},
    time::TimeZone,
    users::UserId,
};

use crate::{
    schema::event_exceptions,
    tables::{
        event_exceptions::{EventExceptionId, EventExceptionKind},
        events::Event,
    },
    users::User,
};

#[derive(Associations, Clone, Debug, Identifiable, Queryable)]
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

impl From<EventException> for inventory::EventException {
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

impl From<inventory::EventException> for EventException {
    fn from(
        inventory::EventException {
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
        }: inventory::EventException,
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
