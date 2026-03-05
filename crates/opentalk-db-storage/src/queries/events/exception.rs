// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;
use opentalk_database::{DbConnection, Result};
use opentalk_types_common::events::EventId;

use crate::{schema::event_exceptions, tables::event_exceptions::EventException};

#[tracing::instrument(err, skip_all)]
pub async fn get_event_exception(
    conn: &mut DbConnection,
    event_id: EventId,
    datetime: DateTime<Utc>,
) -> Result<Option<EventException>> {
    let result = event_exceptions::table
        .filter(
            event_exceptions::event_id
                .eq(event_id)
                .and(event_exceptions::exception_date.eq(datetime)),
        )
        .first(conn)
        .await
        .optional()?;

    Ok(result)
}

#[tracing::instrument(err, skip_all)]
pub async fn get_event_exceptions(
    conn: &mut DbConnection,
    event_id: EventId,
    datetimes: &[&DateTime<Utc>],
) -> Result<Vec<EventException>> {
    let result = event_exceptions::table
        .filter(
            event_exceptions::event_id
                .eq(event_id)
                .and(event_exceptions::exception_date.eq_any(datetimes)),
        )
        .load(conn)
        .await
        .optional()?
        .unwrap_or_default();

    Ok(result)
}

#[tracing::instrument(err, skip_all)]
pub async fn delete_event_exceptions_for_event(
    conn: &mut DbConnection,
    event_id: EventId,
) -> Result<()> {
    diesel::delete(event_exceptions::table)
        .filter(event_exceptions::event_id.eq(event_id))
        .execute(conn)
        .await?;

    Ok(())
}
