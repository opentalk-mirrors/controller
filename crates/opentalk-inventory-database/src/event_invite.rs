// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_db_storage::events::{
    self as db,
    email_invites::{EventEmailInvite, NewEventEmailInvite, UpdateEventEmailInvite},
};
use opentalk_inventory::{
    Event, EventInvite, EventInviteInventory, NewEventInvite, UpdateEventInvite, User,
    error::StorageBackendSnafu,
};
use opentalk_types_common::{
    events::{EventId, invites::EventInviteStatus},
    rooms::RoomId,
    users::UserId,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl EventInviteInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn try_create_event_email_invite(
        &mut self,
        invite: NewEventEmailInvite,
    ) -> Result<Option<EventEmailInvite>> {
        invite
            .try_insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn try_create_event_invite(
        &mut self,
        invite: NewEventInvite,
    ) -> Result<Option<EventInvite>> {
        Ok(db::NewEventInvite::from(invite)
            .try_insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .map(Into::into))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_invites_paginated(
        &mut self,
        event_id: EventId,
        per_page: i64,
        page: i64,
        filter_by_status: Option<EventInviteStatus>,
    ) -> Result<(Vec<(EventInvite, User)>, i64)> {
        let (items, overall) = db::EventInvite::get_for_event_paginated(
            &mut self.inner,
            event_id,
            per_page,
            page,
            filter_by_status,
        )
        .await
        .context(StorageBackendSnafu)?;
        Ok((
            items
                .into_iter()
                .map(|(invite, user)| (invite.into(), user.into()))
                .collect(),
            overall,
        ))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_email_invites_paginated(
        &mut self,
        event_id: EventId,
        per_page: i64,
        page: i64,
    ) -> Result<(Vec<EventEmailInvite>, i64)> {
        EventEmailInvite::get_for_event_paginated(&mut self.inner, event_id, per_page, page)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_invite_for_user_and_room(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Option<EventInvite>> {
        Ok(
            db::EventInvite::get_for_user_and_room(&mut self.inner, user_id, room_id)
                .await
                .context(StorageBackendSnafu)?
                .map(Into::into),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_user_invites_for_events(
        &mut self,
        events: &[&Event],
    ) -> Result<Vec<Vec<(EventInvite, User)>>> {
        let events = events
            .iter()
            .cloned()
            .map(opentalk_db_storage::events::Event::from)
            .collect::<Vec<opentalk_db_storage::events::Event>>();
        let events = events
            .iter()
            .collect::<Vec<&opentalk_db_storage::events::Event>>();
        let invites = db::EventInvite::get_for_events(&mut self.inner, &events)
            .await
            .context(StorageBackendSnafu)?;
        Ok(invites
            .into_iter()
            .map(|items| {
                items
                    .into_iter()
                    .map(|(invite, user)| (invite.into(), user.into()))
                    .collect()
            })
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_email_invites_for_events(
        &mut self,
        events: &[&Event],
    ) -> Result<Vec<Vec<EventEmailInvite>>> {
        let events = events
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<opentalk_db_storage::events::Event>>();
        let events = events.iter().collect::<Vec<&_>>();
        EventEmailInvite::get_for_events(&mut self.inner, &events)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_email_invites_pending_for_user(
        &mut self,
        user_id: UserId,
    ) -> Result<Vec<EventInvite>> {
        Ok(
            db::EventInvite::get_pending_for_user(&mut self.inner, user_id)
                .await
                .context(StorageBackendSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_invite_by_invitee(
        &mut self,
        event_id: EventId,
        user_id: UserId,
    ) -> Result<EventInvite> {
        Ok(
            db::EventInvite::delete_by_invitee(&mut self.inner, event_id, user_id)
                .await
                .context(StorageBackendSnafu)?
                .into(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_invite_by_email(
        &mut self,
        event_id: EventId,
        email: &str,
    ) -> Result<EventEmailInvite> {
        EventEmailInvite::delete(&mut self.inner, &event_id, email)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_event_user_invite(
        &mut self,
        event_id: EventId,
        user_id: UserId,
        event_invite: UpdateEventInvite,
    ) -> Result<EventInvite> {
        Ok(db::UpdateEventInvite::from(event_invite)
            .apply(&mut self.inner, user_id, event_id)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_event_email_invite(
        &mut self,
        event_id: EventId,
        email: &str,
        event_invite: UpdateEventEmailInvite,
    ) -> Result<EventEmailInvite> {
        event_invite
            .apply(&mut self.inner, email, event_id)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn migrate_event_email_invites_to_user_invites(
        &mut self,
        user: User,
    ) -> Result<Vec<(EventId, RoomId)>> {
        EventEmailInvite::migrate_to_user_invites(&mut self.inner, &user.into())
            .await
            .context(StorageBackendSnafu)
    }
}
