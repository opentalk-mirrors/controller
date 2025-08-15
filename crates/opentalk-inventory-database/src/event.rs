// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::collections::BTreeSet;

use opentalk_db_storage::{
    events::{self as db, EventFavorite, GetEventsCursor, NewEventFavorite},
    rooms::Room,
    sip_configs::SipConfig,
    tariffs::Tariff,
    users::User,
};
use opentalk_inventory::{
    Event, EventException, EventExceptionId, EventInventory, EventInvite, EventSharedFolder,
    EventTrainingParticipationReportParameterSet, NewEvent, NewEventException, UpdateEvent,
    UpdateEventException, error::StorageBackendSnafu,
};
use opentalk_types_common::{
    events::{EventId, invites::EventInviteStatus},
    rooms::RoomId,
    time::Timestamp,
    training_participation_report::TrainingParticipationReportParameterSet,
    users::UserId,
};
use snafu::ResultExt as _;

use crate::{DatabaseConnection, Result};

#[async_trait::async_trait]
impl EventInventory for DatabaseConnection {
    #[tracing::instrument(err, skip_all)]
    async fn create_event(&mut self, new_event: NewEvent) -> Result<Event> {
        Ok(db::NewEvent::from(new_event)
            .insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_event(&mut self, event_id: EventId, event: UpdateEvent) -> Result<Event> {
        Ok(db::UpdateEvent::from(event)
            .apply(&mut self.inner, event_id)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event(&mut self, event_id: EventId) -> Result<Event> {
        Ok(db::Event::get(&mut self.inner, event_id)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_for_room(&mut self, room_id: RoomId) -> Result<Option<Event>> {
        Ok(db::Event::get_for_room(&mut self.inner, room_id)
            .await
            .context(StorageBackendSnafu)?
            .map(Into::into))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_id_for_room(&mut self, room_id: RoomId) -> Result<Option<EventId>> {
        Ok(db::Event::get_id_for_room(&mut self.inner, room_id)
            .await
            .context(StorageBackendSnafu)?)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_with_room_and_sip_config(
        &mut self,
        event_id: EventId,
    ) -> Result<(Event, Room, Option<SipConfig>)> {
        let (event, room, sip_config) = db::Event::get_with_room(&mut self.inner, event_id)
            .await
            .context(StorageBackendSnafu)?;
        Ok((event.into(), room, sip_config))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_with_related_items(
        &mut self,
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
        let (
            event,
            invite,
            room,
            sip_config,
            is_favourite,
            shared_folder,
            tariff,
            training_participation_report_parameter_set,
        ) = db::Event::get_with_related_items(&mut self.inner, user_id, event_id)
            .await
            .context(StorageBackendSnafu)?;
        Ok((
            event.into(),
            invite.map(Into::into),
            room,
            sip_config,
            is_favourite,
            shared_folder.map(Into::into),
            tariff,
            training_participation_report_parameter_set.map(Into::into),
        ))
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_events_updated_by_user(&mut self, user_id: UserId) -> Result<Vec<Event>> {
        Ok(db::Event::get_all_updated_by_user(&mut self.inner, user_id)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_adhoc_event_ids_with_room_ids_created_before(
        &mut self,
        created_before: Timestamp,
    ) -> Result<Vec<(EventId, RoomId)>> {
        db::Event::get_all_adhoc_created_before_including_rooms(
            &mut self.inner,
            created_before.into(),
        )
        .await
        .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_scheduled_event_ids_with_room_ids_ended_before(
        &mut self,
        ended_before: Timestamp,
    ) -> Result<Vec<(EventId, RoomId)>> {
        db::Event::get_all_that_ended_before_including_rooms(&mut self.inner, ended_before.into())
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_event_ids_with_room_ids_created_by_user(
        &mut self,
        user_id: UserId,
    ) -> Result<Vec<(EventId, RoomId)>> {
        db::Event::get_all_for_creator_including_rooms(&mut self.inner, user_id)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_finite_recurring_events(&mut self) -> Result<Vec<Event>> {
        Ok(db::Event::get_all_finite_recurring(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_events_for_user_paginated(
        &mut self,
        user: &User,
        only_favorites: bool,
        invite_status_filter: BTreeSet<EventInviteStatus>,
        time_min: Option<Timestamp>,
        time_max: Option<Timestamp>,
        created_before: Option<Timestamp>,
        created_after: Option<Timestamp>,
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
        let items = db::Event::get_all_for_user_paginated(
            &mut self.inner,
            user,
            only_favorites,
            Vec::from_iter(invite_status_filter),
            time_min.map(Into::into),
            time_max.map(Into::into),
            created_before.map(Into::into),
            created_after.map(Into::into),
            adhoc,
            time_independent,
            cursor,
            limit,
        )
        .await
        .context(StorageBackendSnafu)?;
        Ok(items
            .into_iter()
            .map(
                |(
                    event,
                    invite,
                    room,
                    sip_config,
                    exceptions,
                    is_favorite,
                    shared_folder,
                    tariff,
                    training_participation_report_parameters,
                )| {
                    let exceptions = exceptions.into_iter().map(EventException::from).collect();
                    (
                        event.into(),
                        invite.map(Into::into),
                        room,
                        sip_config,
                        exceptions,
                        is_favorite,
                        shared_folder.map(Into::into),
                        tariff,
                        training_participation_report_parameters,
                    )
                },
            )
            .collect())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_event_ids_with_creator_id(&mut self) -> Result<Vec<(EventId, UserId)>> {
        db::Event::get_all_with_creator(&mut self.inner)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_all_event_ids_with_room_ids_and_invitee_ids(
        &mut self,
    ) -> Result<Vec<(EventId, RoomId, UserId)>> {
        db::Event::get_all_with_invitee(&mut self.inner)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn create_event_exception(
        &mut self,
        event_exception: NewEventException,
    ) -> Result<EventException> {
        Ok(db::NewEventException::from(event_exception)
            .insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_exceptions(
        &mut self,
        event_id: EventId,
        timestamps: &[Timestamp],
    ) -> Result<Vec<EventException>> {
        let timestamps: Vec<&_> = timestamps.iter().map(|v| v.as_ref()).collect();
        Ok(
            db::EventException::get_all_for_event(&mut self.inner, event_id, &timestamps)
                .await
                .context(StorageBackendSnafu)?
                .into_iter()
                .map(Into::into)
                .collect(),
        )
    }

    #[tracing::instrument(err, skip_all)]
    async fn get_event_exception(
        &mut self,
        event_id: EventId,
        instance_id_timestamp: Timestamp,
    ) -> Result<Option<EventException>> {
        Ok(db::EventException::get_for_event(
            &mut self.inner,
            event_id,
            instance_id_timestamp.into(),
        )
        .await
        .context(StorageBackendSnafu)?
        .map(Into::into))
    }

    #[tracing::instrument(err, skip_all)]
    async fn update_event_exception(
        &mut self,
        event_exception_id: EventExceptionId,
        event_exception: UpdateEventException,
    ) -> Result<EventException> {
        Ok(db::UpdateEventException::from(event_exception)
            .apply(&mut self.inner, event_exception_id.into())
            .await
            .context(StorageBackendSnafu)?
            .into())
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_exceptions_for_event(&mut self, event_id: EventId) -> Result<()> {
        db::EventException::delete_all_for_event(&mut self.inner, event_id)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_for_room(&mut self, room_id: RoomId) -> Result<()> {
        db::Event::delete_for_room(&mut self.inner, room_id)
            .await
            .context(StorageBackendSnafu)
    }

    #[tracing::instrument(err, skip_all)]
    async fn create_event_favorite_for_user(
        &mut self,
        event_id: EventId,
        user_id: UserId,
    ) -> Result<bool> {
        NewEventFavorite { event_id, user_id }
            .try_insert(&mut self.inner)
            .await
            .context(StorageBackendSnafu)
            .map(|v| v.is_some())
    }

    #[tracing::instrument(err, skip_all)]
    async fn delete_event_favorite_for_user(
        &mut self,
        event_id: EventId,
        user_id: UserId,
    ) -> Result<bool> {
        EventFavorite::delete_by_id(&mut self.inner, user_id, event_id)
            .await
            .context(StorageBackendSnafu)
    }
}
