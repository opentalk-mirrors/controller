// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl, scoped_futures::ScopedFutureExt};
use opentalk_database::{DbConnection, Result};
use opentalk_inventory as inventory;
use opentalk_types_common::{
    events::{EventId, invites::EmailInviteRole},
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomId,
    users::UserId,
};

use crate::{
    paginate::Paginate as _,
    schema::{event_email_invites, event_invites, events},
    tables::{event_invites::NewEventInvite, events::Event},
    users::User,
};

#[derive(Debug, Associations, Identifiable, Queryable)]
#[diesel(table_name = event_email_invites)]
#[diesel(primary_key(event_id, email))]
#[diesel(belongs_to(Event))]
pub struct EventEmailInvite {
    pub event_id: EventId,
    pub email: String,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub role: EmailInviteRole,
}

impl From<EventEmailInvite> for inventory::EventEmailInvite {
    fn from(
        EventEmailInvite {
            event_id,
            email,
            created_by,
            created_at,
            role,
        }: EventEmailInvite,
    ) -> Self {
        Self {
            event_id,
            email,
            created_by,
            created_at: created_at.into(),
            role,
        }
    }
}

impl From<inventory::EventEmailInvite> for EventEmailInvite {
    fn from(
        inventory::EventEmailInvite {
            event_id,
            email,
            created_by,
            created_at,
            role,
        }: inventory::EventEmailInvite,
    ) -> Self {
        Self {
            event_id,
            email,
            created_by,
            created_at: created_at.into(),
            role,
        }
    }
}

impl EventEmailInvite {
    pub async fn migrate_to_user_invites(
        conn: &mut DbConnection,
        user: &User,
    ) -> Result<Vec<(EventId, RoomId)>> {
        conn.transaction(|conn| {
            async move {
                let email_invites_with_room: Vec<(EventEmailInvite, RoomId)> =
                    event_email_invites::table
                        .filter(event_email_invites::email.eq(&user.email))
                        .inner_join(events::table)
                        .filter(events::tenant_id.eq(user.tenant_id))
                        .select((event_email_invites::all_columns, events::room))
                        .load(conn)
                        .await?;

                if email_invites_with_room.is_empty() {
                    return Ok(vec![]);
                }

                let event_ids = email_invites_with_room
                    .iter()
                    .map(|(email_invite, room_id)| (email_invite.event_id, *room_id))
                    .collect();

                let new_invites: Vec<_> = email_invites_with_room
                    .into_iter()
                    .map(|(email_invite, _)| NewEventInvite {
                        event_id: email_invite.event_id,
                        invitee: user.id,
                        role: email_invite.role.into(),
                        created_by: email_invite.created_by,
                        created_at: Some(email_invite.created_at),
                    })
                    .collect();

                diesel::insert_into(event_invites::table)
                    .values(new_invites)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;

                diesel::delete(
                    event_email_invites::table.filter(
                        event_email_invites::email.eq(&user.email).and(
                            event_email_invites::event_id.eq_any(
                                events::table
                                    .filter(events::tenant_id.eq(user.tenant_id))
                                    .select(events::id),
                            ),
                        ),
                    ),
                )
                .execute(conn)
                .await?;

                Ok(event_ids)
            }
            .scope_boxed()
        })
        .await
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_events(
        conn: &mut DbConnection,
        events: &[&Event],
    ) -> Result<Vec<Vec<EventEmailInvite>>> {
        let invites: Vec<EventEmailInvite> =
            EventEmailInvite::belonging_to(events).load(conn).await?;

        let invites_by_event: Vec<Vec<EventEmailInvite>> = invites.grouped_by(events);
        Ok(invites_by_event)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn delete(
        conn: &mut DbConnection,
        event_id: &EventId,
        email: &str,
    ) -> Result<EventEmailInvite> {
        let query = diesel::delete(event_email_invites::table)
            .filter(
                event_email_invites::event_id
                    .eq(event_id)
                    .and(event_email_invites::email.eq(email)),
            )
            .returning(event_email_invites::all_columns);

        let invite = query.get_result(conn).await?;

        Ok(invite)
    }

    #[tracing::instrument(err, skip_all)]
    pub async fn get_for_event_paginated(
        conn: &mut DbConnection,
        event_id: EventId,
        limit: PageSize,
        page: Page,
    ) -> Result<(Vec<EventEmailInvite>, ItemCount)> {
        let query = event_email_invites::table
            .filter(event_email_invites::columns::event_id.eq(event_id))
            .order(event_email_invites::created_at.desc())
            .then_order_by(event_email_invites::created_by.desc())
            .then_order_by(event_email_invites::email.desc())
            .paginate_by(limit, page);

        let invites: (Vec<EventEmailInvite>, ItemCount) = query.load_and_count(conn).await?;

        Ok(invites)
    }
}
