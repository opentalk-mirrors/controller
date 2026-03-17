// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Handles events

use std::{cmp::Ordering, collections::BTreeSet, pin::Pin, sync::Arc};

use chrono::{DateTime, Datelike, NaiveTime, Utc};
use chrono_tz::Tz;
use futures_core::Stream;
use futures_util::{TryStreamExt, pin_mut, stream::StreamExt};
use kustos::{
    AccessMethod, Resource,
    policies_builder::{GrantingAccess, PoliciesBuilder},
    prelude::IsSubject,
};
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::Settings;
use opentalk_controller_utils::{
    CaptureApiError,
    deletion::{Deleter, EventDeleter},
};
use opentalk_inventory::{
    Event, EventEmailInvite, EventException, EventExceptionKind, EventInvite, EventSharedFolder,
    EventTrainingParticipationReportParameterSet, GetEventExceptionsCursor, GetEventsCursor,
    Inventory, InventoryProvider, NewEvent, NewEventDate, NewEventRecurrence, NewRoom,
    NewRoomSipConfig, Room, RoomSipConfig, Tariff, Tenant, UpdateEvent, UpdateEventDate,
    UpdateEventRecurrence, UpdateEventTrainingParticipationReportParameterSet, UpdateRoom, User,
    transaction,
};
use opentalk_keycloak_admin::KeycloakAdminClient;
use opentalk_roomserver_types::room_parameters_patch::RoomParametersPatch;
use opentalk_types_api_v1::{
    error::ApiError,
    events::{
        CallInInfo, DeleteEventsQuery, EmailOnlyUser, EventAndInstanceId, EventDate, EventDateKind,
        EventExceptionResource, EventInvitee, EventInviteeProfile, EventOptionsQuery,
        EventOrException, EventResource, EventResourceDate, EventResourceDateKind, EventRoomInfo,
        EventStatus, ExceptionMarker, GetEventQuery, GetEventsCursorData, GetEventsQuery,
        PatchEventBody, PatchEventDateKind, PatchEventQuery, PostEventsBody,
        PublicInviteUserProfile, TimeDependentMarker,
    },
    pagination::Cursor,
    users::PublicUserProfile,
};
use opentalk_types_common::{
    events::{EventDescription, EventId, EventTitle, invites::EventInviteStatus},
    features::CALL_IN_FEATURE_ID,
    modules::DEFAULT_MODULE_ID,
    pagination::{ItemCount, Page, PageSize},
    rooms::RoomPassword,
    shared_folders::SharedFolder,
    streaming::{RoomStreamingTarget, StreamingTarget},
    tariffs::TariffResource,
    time::{DateTimeTz, RecurrencePattern, TimeZone, Timestamp},
    training_participation_report::TrainingParticipationReportParameterSet,
    users::Language,
};
use rrule::{Frequency, RRuleSet};
use scoped_futures::ScopedFutureExt as _;
use snafu::Report;

use crate::{
    ControllerBackend, ToUserProfile,
    controller_backend::{
        RoomsPoliciesBuilderExt, delete_shared_folders, put_shared_folder,
        utils::interweave_result_streams,
    },
    email_to_libravatar_url,
    events::{
        enrich_from_optional_user_search, enrich_invitees_from_optional_user_search,
        get_invited_mail_recipients_for_event,
        notifications::{UpdateNotificationValues, notify_invitees_about_update},
        shared_folder_for_user,
    },
    services::{MailRecipient, MailService, RegisteredMailRecipient},
    user_profiles::GetUserProfilesBatched,
};

mod favorites;
pub(crate) mod instances;
pub(crate) mod invites;
pub(crate) mod shared_folder;

const LOCAL_DT_FORMAT: &str = "%Y%m%dT%H%M%S";
const ONE_HUNDRED_YEARS_IN_DAYS: usize = 36525;

#[allow(clippy::large_enum_variant, clippy::type_complexity)]
enum InternalEventOrException {
    Event(
        (
            Event,
            Option<EventInvite>,
            Room,
            Option<RoomSipConfig>,
            bool,
            Option<EventSharedFolder>,
            Tariff,
            Option<TrainingParticipationReportParameterSet>,
        ),
    ),
    Exception((EventException, Event)),
}

impl ControllerBackend {
    pub(crate) async fn new_event(
        &self,
        current_user: RequestUser,
        event: PostEventsBody,
        query: EventOptionsQuery,
    ) -> Result<EventResource, CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let current_user = inventory.get_user(current_user.id).await?;
        let tariff = self.get_tariff_for_user(current_user.id).await?;

        let transaction_settings = settings.clone();
        let (event_resource, mail_resource) = transaction(inventory.as_mut(), |inventory| {
            let tariff = tariff.clone();
            async move {
                // simplify logic by splitting the event creation
                // into two paths: time independent and time dependent
                let (mut event_resource, mail_resource) = match event {
                    PostEventsBody {
                        title,
                        description,
                        password,
                        waiting_room,
                        e2e_encryption,
                        is_adhoc,
                        streaming_targets,
                        has_shared_folder: _,
                        show_meeting_details,
                        training_participation_report,
                        date:
                            EventDateKind::TimeIndependent {
                                is_time_independent: _,
                            },
                    } => {
                        create_time_independent_event(
                            &transaction_settings,
                            inventory,
                            current_user,
                            &tariff,
                            title,
                            description,
                            password,
                            waiting_room,
                            e2e_encryption,
                            is_adhoc,
                            streaming_targets,
                            show_meeting_details,
                            query,
                            training_participation_report,
                        )
                        .await?
                    }
                    PostEventsBody {
                        title,
                        description,
                        password,
                        waiting_room,
                        e2e_encryption,
                        is_adhoc,
                        streaming_targets,
                        has_shared_folder: _,
                        show_meeting_details,
                        training_participation_report,
                        date:
                            EventDateKind::TimeDependent {
                                is_time_independent: _,
                                date:
                                    EventDate {
                                        is_all_day,
                                        starts_at,
                                        ends_at,
                                        recurrence_pattern,
                                    },
                            },
                    } => {
                        create_time_dependent_event(
                            &transaction_settings,
                            inventory,
                            current_user,
                            &tariff,
                            title,
                            description,
                            password,
                            waiting_room,
                            e2e_encryption,
                            is_all_day,
                            starts_at,
                            ends_at,
                            recurrence_pattern,
                            is_adhoc,
                            streaming_targets,
                            show_meeting_details,
                            query,
                            training_participation_report,
                        )
                        .await?
                    }
                };

                if event.has_shared_folder {
                    let (shared_folder, _) =
                        put_shared_folder(&transaction_settings, event_resource.id, inventory)
                            .await?;
                    event_resource.shared_folder = Some(SharedFolder::from(shared_folder));
                }

                Result::<_, CaptureApiError>::Ok((event_resource, mail_resource))
            }
            .scope_boxed()
        })
        .await?;

        drop(inventory);

        let policies = PoliciesBuilder::new()
            .grant_user_access(event_resource.created_by.id)
            .event_read_access(event_resource.id)
            .event_write_access(event_resource.id)
            .room_read_access(event_resource.room.id)
            .room_write_access(event_resource.room.id)
            .finish();

        self.authz.add_policies(policies).await?;

        if let (Some(mail_resource), Some(mail_service)) =
            (mail_resource, self.mail_service.as_ref())
        {
            mail_service
                .send_registered_invite(
                    &settings,
                    mail_resource.current_user.clone(),
                    mail_resource.event,
                    mail_resource.room,
                    &tariff,
                    mail_resource.sip_config,
                    mail_resource.current_user,
                    event_resource.shared_folder.clone(),
                    event_resource.streaming_targets.clone(),
                )
                .await
                .map_err(|e| {
                    log::warn!("Failed to send with MailService: {}", Report::from_error(e));
                    CaptureApiError::from(ApiError::internal())
                })?;
        }

        Ok(event_resource)
    }

    pub(crate) async fn get_events_and_exceptions_interwoven(
        &self,
        current_user: RequestUser,
        query: GetEventsQuery,
    ) -> Result<(Vec<EventOrException>, Option<String>, Option<String>), CaptureApiError> {
        let get_events_cursor = query.after.map(|cursor| {
            GetEventsCursor::new(
                cursor.event_id,
                cursor.event_created_at,
                cursor.event_starts_at,
            )
        });
        let get_event_exceptions_cursor = query.after.map(|cursor| {
            GetEventExceptionsCursor::new(
                cursor.event_id,
                cursor.event_created_at,
                cursor.event_starts_at,
                cursor
                    .instance_id
                    .map(Into::into)
                    .unwrap_or_else(Timestamp::unix_epoch),
            )
        });

        // Get the streams to interweave from their sources.
        let mut inventory = self.inventory_provider.get_inventory().await?;
        let current_user = inventory.get_user(current_user.id).await?;
        let events_stream = inventory
            .get_all_events_for_user_paginated_as_stream(
                current_user.clone(),
                query.favorites,
                BTreeSet::from_iter(query.invite_status.clone()),
                query.time_min,
                query.time_max,
                query.created_before,
                query.created_after,
                query.adhoc,
                query.time_independent,
                get_events_cursor,
            )
            .await?;
        let mut inventory = self.inventory_provider.get_inventory().await?;
        let instances_stream = inventory
            .get_all_event_exceptions_for_user_paginated_as_stream(
                current_user.clone(),
                query.favorites,
                BTreeSet::from_iter(query.invite_status),
                query.time_min,
                query.time_max,
                query.created_before,
                query.created_after,
                query.adhoc,
                query.time_independent,
                get_event_exceptions_cursor,
            )
            .await?;

        // Prepare all streams to interweave (one for the events and one for the event exceptions).
        let events_stream = Box::pin(events_stream.map_ok(InternalEventOrException::Event));
        let instances_stream =
            Box::pin(instances_stream.map_ok(InternalEventOrException::Exception));

        // Interweave the streams
        let streams: Vec<
            Pin<Box<dyn Stream<Item = opentalk_inventory::Result<InternalEventOrException>>>>,
        > = vec![events_stream, instances_stream];
        let stream = interweave_result_streams(streams, compare_inventory_events_or_exceptions);

        // Create the resource vector
        let settings = self.settings_provider.get();
        let vector = self
            .create_events_or_exceptions_vec_from_stream(
                self.inventory_provider.clone(),
                &settings,
                current_user,
                stream,
                query.invitees_max,
                query.per_page,
            )
            .await?;

        // Build a cursor that can be used to fetch the next page
        let ret_cursor_data = vector.last().map(|item| {
            let (event_id, event_created_at, event_starts_at, instance_id) = match item {
                EventOrException::Event(event) => (
                    event.id,
                    event.created_at,
                    event.date.starts_at().cloned(),
                    None,
                ),
                EventOrException::Exception(exception) => (
                    exception.recurring_event_id,
                    exception.created_at,
                    exception.starts_at,
                    Some(exception.instance_id),
                ),
            };

            GetEventsCursorData {
                event_id,
                event_created_at,
                event_starts_at: event_starts_at.map(Into::into),
                instance_id,
            }
        });

        let before = None;
        let after = ret_cursor_data.map(|c| Cursor(c).to_base64());

        Ok((vector, before, after))
    }

    async fn create_events_or_exceptions_vec_from_stream(
        &self,
        inventory_provider: Arc<dyn InventoryProvider>,
        settings: &Settings,
        current_user: User,
        stream: impl Stream<Item = opentalk_inventory::Result<InternalEventOrException>>,
        invitees_max: Option<PageSize>,
        per_page: Option<PageSize>,
    ) -> Result<Vec<EventOrException>, CaptureApiError> {
        let stream = stream.take(per_page.unwrap_or_default().into());
        let mut items: Vec<EventOrException> = Vec::new();

        pin_mut!(stream);
        while let Some(result) = stream.next().await {
            let mut inventory = inventory_provider.get_inventory().await?;

            let item = match result? {
                InternalEventOrException::Event(inventory_item) => {
                    let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
                    let resource = self
                        .build_event_resource(
                            inventory.as_mut(),
                            settings,
                            current_user.clone(),
                            &current_tenant,
                            invitees_max,
                            inventory_item,
                        )
                        .await?;
                    EventOrException::Event(resource)
                }
                InternalEventOrException::Exception(inventory_item) => {
                    let resource = self
                        .build_event_exception_resource(
                            inventory.as_mut(),
                            settings,
                            current_user.clone(),
                            inventory_item,
                        )
                        .await?;
                    EventOrException::Exception(resource)
                }
            };

            items.push(item);
        }

        Ok(items)
    }

    #[allow(clippy::type_complexity)]
    async fn build_event_resource(
        &self,
        inventory: &mut dyn Inventory,
        settings: &Settings,
        current_user: User,
        current_tenant: &Tenant,
        invitees_max: Option<PageSize>,
        (
            event,
            invite,
            room,
            sip_config,
            is_favorite,
            shared_folder,
            tariff,
            training_participation_report,
        ): (
            Event,
            Option<EventInvite>,
            Room,
            Option<RoomSipConfig>,
            bool,
            Option<EventSharedFolder>,
            Tariff,
            Option<TrainingParticipationReportParameterSet>,
        ),
    ) -> Result<EventResource, CaptureApiError> {
        let users = GetUserProfilesBatched::new()
            .add(&event)
            .fetch(settings, inventory)
            .await?;

        // Build list of event invites with user, grouped by events
        let mut invites_with_user = if invitees_max.is_none() {
            // Do not query event invites if invitees_max is zero, instead create dummy value
            Vec::new()
        } else {
            inventory
                .get_event_user_invites_for_events(&[&event])
                .await?
                .first()
                .expect("outer vector must have one item for the event")
                .clone()
        };

        // Build list of additional email event invites, grouped by events
        let mut email_invites = if invitees_max.is_none() {
            // Do not query email event invites if invitees_max is zero, instead create dummy value
            Vec::new()
        } else {
            inventory
                .get_event_email_invites_for_events(&[&event])
                .await?
                .first()
                .expect("outer vector must have one item for the event")
                .clone()
        };

        let created_by = users.get(event.created_by);
        let updated_by = users.get(event.updated_by);

        let invite_status = invite
            .map(|invite| invite.status)
            .unwrap_or(EventInviteStatus::Accepted);

        let invitees_truncated = if let Some(invitees_max) = invitees_max {
            let email_invites_max =
                usize::from(invitees_max).saturating_sub(invites_with_user.len());

            let invitees_truncated =
                (invites_with_user.len() + email_invites.len()) > usize::from(invitees_max);

            invites_with_user.truncate(invitees_max.into());
            email_invites.truncate(email_invites_max);

            invitees_truncated
        } else {
            invites_with_user.clear();
            email_invites.clear();
            false
        };

        let registered_invitees_iter = invites_with_user
            .into_iter()
            .map(|(invite, user)| EventInvitee::from_invite_with_user(invite, user, settings));

        let unregistered_invitees_iter = email_invites
            .into_iter()
            .map(|invite| EventInvitee::from_email_invite(invite, settings));

        let invitees: Vec<_> = registered_invitees_iter
            .chain(unregistered_invitees_iter)
            .collect();

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let tariff = self.build_tariff_resource(&tariff)?;

        let date = event
            .date()
            .map(|date| EventResourceDateKind::TimeDependent {
                is_time_independent: TimeDependentMarker,
                date: event
                    .recurrence()
                    .and_then(|recurrence| {
                        recurrence
                            .recurrence_pattern
                            .parse::<RecurrencePattern>()
                            .ok()
                    })
                    .map(|recurrence_pattern| EventResourceDate::Recurring {
                        is_all_day: date.is_all_day,
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        recurrence_pattern,
                    })
                    .unwrap_or_else(|| EventResourceDate::Single {
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        is_all_day: date.is_all_day,
                    }),
            })
            .unwrap_or(EventResourceDateKind::TIME_INDEPENDENT);

        let invitees = enrich_invitees_from_optional_user_search(
            settings,
            &self.user_search_client,
            current_tenant,
            invitees.clone(),
        )
        .await;

        let resource = EventResource {
            id: event.id,
            created_by,
            created_at: event.created_at,
            updated_by,
            updated_at: event.updated_at,
            title: event.title,
            description: event.description,
            room: EventRoomInfo::from_room(settings, &room, sip_config.as_ref(), &tariff),
            invitees_truncated,
            invitees,
            invite_status,
            is_favorite,
            can_edit,
            shared_folder,
            streaming_targets: Vec::new(),
            show_meeting_details: event.show_meeting_details,
            training_participation_report,
            is_adhoc: event.is_adhoc,
            date,
        };

        Ok(resource)
    }

    async fn build_event_exception_resource(
        &self,
        inventory: &mut dyn Inventory,
        settings: &Settings,
        current_user: User,
        (event_exception, event): (EventException, Event),
    ) -> Result<EventExceptionResource, CaptureApiError> {
        let users = GetUserProfilesBatched::new()
            .add(&event_exception)
            .fetch(settings, inventory)
            .await?;

        let created_by = users.get(event_exception.created_by);
        let can_edit = current_user.can_edit(&event);

        let resource =
            EventExceptionResource::from_inventory(event_exception, created_by, can_edit);

        Ok(resource)
    }

    pub(crate) async fn get_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
        query: GetEventQuery,
    ) -> Result<EventResource, CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let settings = self.settings_provider.get();

        let (
            event,
            invite,
            room,
            sip_config,
            is_favorite,
            shared_folder,
            tariff,
            training_participation_report,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;

        let room_streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

        let (invitees, invitees_truncated) =
            get_invitees_for_event(&settings, inventory.as_mut(), event_id, query.invitees_max)
                .await?;

        let users = GetUserProfilesBatched::new()
            .add(&event)
            .fetch(&settings, inventory.as_mut())
            .await?;

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        drop(inventory);

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let tariff = self.build_tariff_resource(&tariff)?;

        let date = event
            .date()
            .map(|date| EventResourceDateKind::TimeDependent {
                is_time_independent: TimeDependentMarker,
                date: event
                    .recurrence()
                    .and_then(|recurrence| {
                        recurrence
                            .recurrence_pattern
                            .parse::<RecurrencePattern>()
                            .ok()
                    })
                    .map(|recurrence_pattern| EventResourceDate::Recurring {
                        is_all_day: date.is_all_day,
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        recurrence_pattern,
                    })
                    .unwrap_or_else(|| EventResourceDate::Single {
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        is_all_day: date.is_all_day,
                    }),
            })
            .unwrap_or(EventResourceDateKind::TIME_INDEPENDENT);

        let event_resource = EventResource {
            id: event.id,
            title: event.title,
            description: event.description,
            room: EventRoomInfo::from_room(&settings, &room, sip_config.as_ref(), &tariff),
            invitees_truncated,
            invitees,
            created_by: users.get(event.created_by),
            created_at: event.created_at,
            updated_by: users.get(event.updated_by),
            updated_at: event.updated_at,
            invite_status: invite
                .map(|inv| inv.status)
                .unwrap_or(EventInviteStatus::Accepted),
            is_favorite,
            can_edit,
            is_adhoc: event.is_adhoc,
            shared_folder,
            streaming_targets: room_streaming_targets,
            show_meeting_details: event.show_meeting_details,
            training_participation_report: training_participation_report.map(Into::into),
            date,
        };

        let event_resource = EventResource {
            invitees: enrich_invitees_from_optional_user_search(
                &settings,
                &self.user_search_client,
                &current_tenant,
                event_resource.invitees,
            )
            .await,
            ..event_resource
        };

        Ok(event_resource)
    }

    pub(crate) async fn patch_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
        query: PatchEventQuery,
        patch: PatchEventBody,
    ) -> Result<Option<EventResource>, CaptureApiError> {
        if patch.is_empty() {
            return Ok(None);
        }

        let settings = self.settings_provider.get();
        let default_user_language = Language(settings.defaults.user_language.clone());

        let mail_service = (!query.suppress_email_notification)
            .then(|| self.mail_service.as_ref().clone())
            .flatten();

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (
            event,
            invite,
            room,
            sip_config,
            is_favorite,
            shared_folder,
            tariff,
            training_participation_report,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;

        let room = if patch.password.is_some() || patch.waiting_room.is_some() {
            // Update the event's room if at least one of the fields is set
            inventory
                .update_room(
                    event.room,
                    UpdateRoom {
                        password: patch.password.clone(),
                        waiting_room: patch.waiting_room,
                        e2e_encryption: patch.e2e_encryption,
                    },
                )
                .await?
        } else {
            room
        };

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        let created_by = if event.created_by == current_user.id {
            current_user.clone()
        } else {
            inventory.get_user(event.created_by).await?
        };

        let streaming_targets = if let Some(streaming_targets) = patch.streaming_targets.clone() {
            inventory
                .replace_room_streaming_targets(room.id, streaming_targets)
                .await?
        } else {
            inventory.get_room_streaming_targets(room.id).await?
        };

        let shared_folder = match patch.has_shared_folder {
            Some(true) => {
                let (shared_folder, _) =
                    put_shared_folder(&settings, event_id, inventory.as_mut()).await?;
                Some(shared_folder)
            }
            Some(false) => {
                if let Some(folder) = inventory.get_event_shared_folder(event_id).await? {
                    let shared_folders = std::slice::from_ref(&folder);
                    delete_shared_folders(&settings, shared_folders).await?;
                    inventory.delete_shared_folder_by_event_id(event_id).await?;
                }
                None
            }
            None => shared_folder,
        };

        let training_participation_report = match &patch.training_participation_report {
            Some(Some(parameter_set)) => {
                if training_participation_report.is_some() {
                    Some(
                        inventory
                            .update_training_participation_report_parameter_set(
                                event.id,
                                UpdateEventTrainingParticipationReportParameterSet::from(
                                    parameter_set.clone(),
                                ),
                            )
                            .await?,
                    )
                } else {
                    inventory
                        .try_create_event_training_participation_report_parameter_set(
                            EventTrainingParticipationReportParameterSet::from((
                                event.id,
                                parameter_set.clone(),
                            )),
                        )
                        .await?
                }
            }
            Some(None) => {
                inventory
                    .delete_event_training_participation_report_parameter_set(event.id)
                    .await?;
                None
            }
            None => training_participation_report,
        };

        let update_date = if let Some(patch_date) = &patch.date {
            match patch_date {
                PatchEventDateKind::SetTimeDependent { date, .. } => {
                    // The patch changes the event from an time-independent event to a time
                    // dependent event.
                    let recurrence_pattern = date.recurrence_pattern.to_multiline_string();

                    let (duration_secs, ends_at_dt, ends_at_tz) = parse_event_dt_params(
                        date.is_all_day,
                        date.starts_at,
                        date.ends_at,
                        &recurrence_pattern,
                    )?;

                    Some(UpdateEventDate {
                        is_all_day: Some(date.is_all_day),
                        starts_at: Some(date.starts_at.to_datetime_tz()),
                        starts_at_tz: Some(date.starts_at.timezone),
                        ends_at: Some(ends_at_dt),
                        ends_at_tz: Some(ends_at_tz),
                        recurrence: Some(UpdateEventRecurrence {
                            duration_secs,
                            recurrence_pattern,
                        }),
                    })
                }
                PatchEventDateKind::SetTimeIndependent { .. } => {
                    // The patch will change an event to a time-independent event.
                    if event
                        .date
                        .as_ref()
                        .and_then(|date| date.recurrence.as_ref())
                        .is_some()
                    {
                        inventory
                            .delete_event_exceptions_for_event(event.id)
                            .await?;
                    }

                    None
                }
                PatchEventDateKind::PatchTimeDependent { date, .. } => {
                    // The patch modifies an time dependent event.
                    let recurrence_pattern = date.recurrence_pattern.to_multiline_string();

                    let Some(event_date) = event.date() else {
                        return Err(ApiError::internal()
                            .with_message("tried to patch time depedent event without date")
                            .into());
                    };

                    let is_all_day = date.is_all_day.unwrap_or(event_date.is_all_day);

                    let starts_at = date.starts_at.unwrap_or(DateTimeTz {
                        datetime: event_date.starts_at.into(),
                        timezone: event_date.starts_at_tz,
                    });

                    let ends_at = date.ends_at.unwrap_or_else(|| {
                        let (ends_at, ends_at_tz) = event_date.ends_at_of_first_occurrence();
                        DateTimeTz {
                            datetime: ends_at.into(),
                            timezone: ends_at_tz,
                        }
                    });

                    let (duration_secs, ends_at_dt, ends_at_tz) =
                        parse_event_dt_params(is_all_day, starts_at, ends_at, &recurrence_pattern)?;

                    if event.is_recurring() {
                        // Delete all exceptions for recurring events as the patch may modify fields
                        // that influence the timestamps at which instances (occurrences) are
                        // generated, making it impossible to match the exceptions to instances.
                        inventory
                            .delete_event_exceptions_for_event(event.id)
                            .await?;
                    }

                    Some(UpdateEventDate {
                        is_all_day: Some(is_all_day),
                        starts_at: Some(starts_at.to_datetime_tz()),
                        starts_at_tz: Some(starts_at.timezone),
                        ends_at: Some(ends_at_dt),
                        ends_at_tz: Some(ends_at_tz),
                        recurrence: Some(UpdateEventRecurrence {
                            duration_secs,
                            recurrence_pattern,
                        }),
                    })
                }
            }
        } else {
            // The event date did not change regardless if the event was previously time-indepent
            // or not.
            None
        };

        let event = if patch.only_modifies_room() {
            // Special case: If the patch only modifies the password do not update the event.
            event
        } else {
            let update_event = UpdateEvent {
                title: patch.title.clone(),
                description: patch.description,
                updated_by: current_user.id,
                updated_at: Timestamp::now(),
                is_adhoc: patch.is_adhoc,
                show_meeting_details: patch.show_meeting_details,
                date: update_date,
            };

            inventory.update_event(event_id, update_event).await?
        };

        // Update the room parameters of the roomserver (if applicable)
        self.patch_room_parameters(
            room.id,
            RoomParametersPatch {
                password: patch.password,
                title: patch.title,
            },
        )
        .await?;

        let invited_users = get_invited_mail_recipients_for_event(
            inventory.as_mut(),
            event_id,
            default_user_language.clone(),
        )
        .await?;
        let current_user_mail_recipient =
            MailRecipient::Registered(RegisteredMailRecipient::from_inventory_user(
                current_user.clone(),
                default_user_language,
            ));
        let users_to_notify = invited_users
            .into_iter()
            .chain(std::iter::once(current_user_mail_recipient))
            .collect::<Vec<_>>();
        let invite_for_room = inventory
            .get_or_create_valid_invite_for_room(room.id, current_user.id)
            .await?;

        // Add the access policy for the invite code, just in case it has been created by
        // the `Invite::get_first_for_room(…)` call above. That function is not able to
        // add the policy, because it has no access to the `RoomsPoliciesBuilderExt` trait.
        let policies = PoliciesBuilder::new()
            // Grant invitee access
            .grant_invite_access(invite_for_room.invite_code)
            .room_guest_read_access(room.id)
            .finish();
        self.authz.add_policies(policies).await?;

        let (invitees, invitees_truncated) =
            get_invitees_for_event(&settings, inventory.as_mut(), event_id, query.invitees_max)
                .await?;

        drop(inventory);

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let tariff = self.build_tariff_resource(&tariff)?;

        let date = event
            .date()
            .map(|date| EventResourceDateKind::TimeDependent {
                is_time_independent: TimeDependentMarker,
                date: event
                    .recurrence()
                    .and_then(|recurrence| {
                        recurrence
                            .recurrence_pattern
                            .parse::<RecurrencePattern>()
                            .ok()
                    })
                    .map(|recurrence_pattern| EventResourceDate::Recurring {
                        is_all_day: date.is_all_day,
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        recurrence_pattern,
                    })
                    .unwrap_or_else(|| EventResourceDate::Single {
                        starts_at: DateTimeTz {
                            datetime: date.starts_at.into(),
                            timezone: date.starts_at_tz,
                        },
                        ends_at: DateTimeTz {
                            datetime: date.ends_at.into(),
                            timezone: date.ends_at_tz,
                        },
                        is_all_day: date.is_all_day,
                    }),
            })
            .unwrap_or(EventResourceDateKind::TIME_INDEPENDENT);

        let event_resource = EventResource {
            id: event.id,
            created_by: created_by.to_public_user_profile(&settings),
            created_at: event.created_at,
            updated_by: current_user.to_public_user_profile(&settings),
            updated_at: event.updated_at,
            title: event.title.clone(),
            description: event.description.clone(),
            room: EventRoomInfo::from_room(&settings, &room, sip_config.as_ref(), &tariff),
            invitees_truncated,
            invitees,
            invite_status: invite
                .map(|inv| inv.status)
                .unwrap_or(EventInviteStatus::Accepted),
            is_favorite,
            can_edit,
            is_adhoc: event.is_adhoc,
            shared_folder: shared_folder.clone(),
            streaming_targets: streaming_targets.clone(),
            show_meeting_details: event.show_meeting_details,
            training_participation_report: training_participation_report.map(Into::into),
            date,
        };

        let event_resource = EventResource {
            invitees: enrich_invitees_from_optional_user_search(
                &settings,
                &self.user_search_client,
                &current_tenant,
                event_resource.invitees,
            )
            .await,
            ..event_resource
        };

        let notification_values = UpdateNotificationValues {
            tenant: current_tenant,
            created_by,
            event,
            event_exception: None,
            room,
            sip_config,
            users_to_notify,
            invite_for_room,
        };

        if let Some(mail_service) = &mail_service {
            notify_invitees_about_update(
                &settings,
                &tariff,
                notification_values,
                mail_service,
                &self.user_search_client,
                shared_folder,
                streaming_targets,
            )
            .await;
        }

        Ok(Some(event_resource))
    }

    pub(crate) async fn delete_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
        DeleteEventsQuery {
            suppress_email_notification,
            force_delete_reference_if_external_services_fail,
        }: DeleteEventsQuery,
    ) -> Result<(), CaptureApiError> {
        let settings = self.settings_provider.get();
        let default_user_language = Language(settings.defaults.user_language.clone());
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let mail_service = (!suppress_email_notification)
            .then(|| self.mail_service.as_ref().clone())
            .flatten();

        // TODO(w.rabl) Further DB access optimization (replacing call to get_with_invite_and_room)?
        let (
            event,
            _invite,
            room,
            sip_config,
            _is_favorite,
            shared_folder,
            tariff,
            _training_participation_report,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;
        let room_tariff = self.build_tariff_resource(&tariff)?;

        let streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        let current_user_id = current_user.id;
        let created_by = if event.created_by == current_user_id {
            current_user
        } else {
            inventory.get_user(event.created_by).await?
        };

        let invited_users = get_invited_mail_recipients_for_event(
            inventory.as_mut(),
            event_id,
            default_user_language.clone(),
        )
        .await?;
        let created_by_mail_recipient = MailRecipient::Registered(
            RegisteredMailRecipient::from_inventory_user(created_by.clone(), default_user_language),
        );
        let users_to_notify = invited_users
            .into_iter()
            .chain(std::iter::once(created_by_mail_recipient))
            .collect::<Vec<_>>();

        let deleter = EventDeleter::new(event_id, force_delete_reference_if_external_services_fail);
        deleter
            .perform(
                log::logger(),
                inventory.as_mut(),
                &self.authz,
                Some(current_user_id),
                self.exchange_handle.clone(),
                &settings,
                &self.storage,
            )
            .await?;

        drop(inventory);

        if let Some(mail_service) = &mail_service {
            let notification_values = CancellationNotificationValues {
                tenant: current_tenant,
                created_by,
                event,
                room,
                sip_config,
                users_to_notify,
                shared_folder: shared_folder.map(SharedFolder::from),
                streaming_targets,
            };

            notify_invitees_about_delete(
                &settings,
                &room_tariff,
                notification_values,
                mail_service,
                &self.user_search_client,
            )
            .await;
        }

        Ok(())
    }
}

pub(crate) trait DateTimeTzFromInventory: Sized {
    fn maybe_from_inventory(utc_dt: Option<Timestamp>, tz: Option<TimeZone>) -> Option<Self>;
    fn to_datetime_tz(self) -> DateTime<Tz>;
}

impl DateTimeTzFromInventory for DateTimeTz {
    /// Create a [`DateTimeTz`] from the database results
    ///
    /// Returns None if any of them are none.
    ///
    /// Only used to exceptions. To get the correct starts_at/ends_at [`DateTimeTz`] values
    /// [`DateTimeTz::starts_at_of`] and [`DateTimeTz::ends_at_of`] is used
    fn maybe_from_inventory(utc_dt: Option<Timestamp>, tz: Option<TimeZone>) -> Option<Self> {
        if let (Some(utc_dt), Some(tz)) = (utc_dt, tz) {
            Some(Self {
                datetime: utc_dt.into(),
                timezone: tz,
            })
        } else {
            None
        }
    }

    /// Combine the inner UTC time with the inner timezone
    fn to_datetime_tz(self) -> DateTime<Tz> {
        self.datetime.with_timezone(self.timezone.as_ref())
    }
}

trait EventResourceExt {
    fn from_inventory(
        exception: EventException,
        created_by: PublicUserProfile,
        can_edit: bool,
    ) -> Self;
}

impl EventResourceExt for EventExceptionResource {
    fn from_inventory(
        exception: EventException,
        created_by: PublicUserProfile,
        can_edit: bool,
    ) -> Self {
        Self {
            id: EventAndInstanceId(exception.event_id, exception.exception_date.into()),
            recurring_event_id: exception.event_id,
            instance_id: exception.exception_date.into(),
            created_by: created_by.clone(),
            created_at: exception.created_at,
            updated_by: created_by,
            updated_at: exception.created_at,
            title: exception.title,
            description: exception.description,
            is_all_day: exception.is_all_day,
            starts_at: DateTimeTz::maybe_from_inventory(
                exception.starts_at,
                exception.starts_at_tz,
            ),
            ends_at: DateTimeTz::maybe_from_inventory(exception.ends_at, exception.ends_at_tz),
            original_starts_at: DateTimeTz {
                datetime: exception.exception_date.into(),
                timezone: exception.exception_date_tz,
            },
            type_: ExceptionMarker::Exception,
            status: match exception.kind {
                EventExceptionKind::Modified => EventStatus::Ok,
                EventExceptionKind::Cancelled => EventStatus::Cancelled,
            },
            can_edit,
        }
    }
}

trait EventInviteeExt {
    fn from_invite_with_user(invite: EventInvite, user: User, settings: &Settings) -> Self;
    fn from_email_invite(invite: EventEmailInvite, settings: &Settings) -> Self;
}

impl EventInviteeExt for EventInvitee {
    fn from_invite_with_user(invite: EventInvite, user: User, settings: &Settings) -> EventInvitee {
        EventInvitee {
            profile: EventInviteeProfile::Registered(PublicInviteUserProfile {
                user_profile: user.to_public_user_profile(settings),
                role: invite.role,
            }),
            status: invite.status,
        }
    }

    fn from_email_invite(invite: EventEmailInvite, settings: &Settings) -> EventInvitee {
        let avatar_url = email_to_libravatar_url(&settings.avatar.libravatar_url, &invite.email);
        EventInvitee {
            profile: EventInviteeProfile::Email(EmailOnlyUser {
                email: invite.email,
                avatar_url,
            }),
            status: EventInviteStatus::Pending,
        }
    }
}

trait EventRoomInfoExt {
    fn from_room(
        settings: &Settings,
        room: &Room,
        sip_config: Option<&RoomSipConfig>,
        tariff: &TariffResource,
    ) -> Self;
}

impl EventRoomInfoExt for EventRoomInfo {
    /// Create a new [`EventRoomInfo`]
    ///
    /// The [`EventRoomInfo`] also contains a [`CallInInfo`] if the following conditions are true:
    /// - a telephone number is configured in the call-in settings
    /// - a [`SipConfig`] is provided
    /// - the `CallIn` feature is not disabled in the settings
    fn from_room(
        settings: &Settings,
        room: &Room,
        sip_config: Option<&RoomSipConfig>,
        tariff: &TariffResource,
    ) -> Self {
        let call_in_feature_is_enabled = tariff
            .has_feature_enabled(&DEFAULT_MODULE_ID, &CALL_IN_FEATURE_ID)
            && !room.e2e_encryption;

        let mut call_in = None;

        if call_in_feature_is_enabled
            && let (Some(call_in_config), Some(sip_config)) = (&settings.call_in, sip_config)
        {
            call_in = Some(CallInInfo {
                tel: call_in_config.tel.clone(),
                uri: None,
                id: sip_config.sip_id.to_string(),
                password: sip_config.password.to_string(),
            });
        }

        Self {
            id: room.id,
            password: room.password.clone(),
            waiting_room: room.waiting_room,
            e2e_encryption: room.e2e_encryption,
            call_in,
        }
    }
}

async fn store_event_streaming_targets(
    inventory: &mut dyn Inventory,
    event_id: EventId,
    streaming_targets: Vec<StreamingTarget>,
) -> Result<Vec<RoomStreamingTarget>, CaptureApiError> {
    let room_id = inventory.get_event(event_id).await?.room;

    let mut room_streaming_targets: Vec<RoomStreamingTarget> = Vec::new();
    for streaming_target in streaming_targets {
        room_streaming_targets.push(
            inventory
                .create_room_streaming_target(room_id, streaming_target)
                .await?,
        );
    }

    Ok(room_streaming_targets)
}

async fn store_training_participation_report(
    inventory: &mut dyn Inventory,
    event_id: EventId,
    TrainingParticipationReportParameterSet {
        initial_checkpoint_delay,
        checkpoint_interval,
    }: TrainingParticipationReportParameterSet,
) -> Result<Option<TrainingParticipationReportParameterSet>, CaptureApiError> {
    let inserted = inventory
        .try_create_event_training_participation_report_parameter_set(
            EventTrainingParticipationReportParameterSet {
                event_id,
                initial_checkpoint_delay,
                checkpoint_interval,
            },
        )
        .await?;

    Ok(inserted.map(Into::into))
}

struct MailResource {
    pub current_user: User,
    pub event: Event,
    pub room: Room,
    pub sip_config: Option<RoomSipConfig>,
}

/// Part of `POST /events` endpoint
#[allow(clippy::too_many_arguments)]
async fn create_time_independent_event(
    settings: &Settings,
    inventory: &mut dyn Inventory,
    current_user: User,
    user_tariff: &TariffResource,
    title: EventTitle,
    description: EventDescription,
    password: Option<RoomPassword>,
    waiting_room: bool,
    e2e_encryption: bool,
    is_adhoc: bool,
    streaming_targets: Vec<StreamingTarget>,
    show_meeting_details: bool,
    query: EventOptionsQuery,
    training_participation_report: Option<TrainingParticipationReportParameterSet>,
) -> Result<(EventResource, Option<MailResource>), CaptureApiError> {
    let room = inventory
        .create_room(NewRoom {
            created_by: current_user.id,
            password,
            waiting_room,
            tenant_id: current_user.tenant_id,
            e2e_encryption,
        })
        .await?;

    let sip_config = inventory
        .create_room_sip_config(NewRoomSipConfig::new(room.id, false))
        .await?;

    let event = inventory
        .create_event(NewEvent {
            title,
            description,
            room: room.id,
            created_by: current_user.id,
            updated_by: current_user.id,
            is_adhoc,
            show_meeting_details,
            tenant_id: current_user.tenant_id,
            date: None,
        })
        .await?;

    let streaming_targets =
        store_event_streaming_targets(inventory, event.id, streaming_targets).await?;

    let training_participation_report = if let Some(parameters) = training_participation_report {
        store_training_participation_report(inventory, event.id, parameters).await?
    } else {
        None
    };

    let suppress_email_notification = is_adhoc || query.suppress_email_notification;

    let event_resource = EventResource {
        id: event.id,
        title: event.title.clone(),
        description: event.description.clone(),
        room: EventRoomInfo::from_room(settings, &room, Some(&sip_config), user_tariff),
        invitees_truncated: false,
        invitees: vec![],
        created_by: current_user.to_public_user_profile(settings),
        created_at: event.created_at,
        updated_by: current_user.to_public_user_profile(settings),
        updated_at: event.updated_at,
        invite_status: EventInviteStatus::Accepted,
        is_favorite: false,
        can_edit: true, // just created by the current user
        is_adhoc,
        shared_folder: None,
        streaming_targets,
        show_meeting_details,
        training_participation_report,
        date: EventResourceDateKind::TIME_INDEPENDENT,
    };

    let mail_resource = (!suppress_email_notification).then_some(MailResource {
        current_user,
        event,
        room,
        sip_config: Some(sip_config),
    });

    Ok((event_resource, mail_resource))
}

/// Part of `POST /events` endpoint
#[allow(clippy::too_many_arguments)]
async fn create_time_dependent_event(
    settings: &Settings,
    inventory: &mut dyn Inventory,
    current_user: User,
    user_tariff: &TariffResource,
    title: EventTitle,
    description: EventDescription,
    password: Option<RoomPassword>,
    waiting_room: bool,
    e2e_encryption: bool,
    is_all_day: bool,
    starts_at: DateTimeTz,
    ends_at: DateTimeTz,
    recurrence_pattern: RecurrencePattern,
    is_adhoc: bool,
    streaming_targets: Vec<StreamingTarget>,
    show_meeting_details: bool,
    query: EventOptionsQuery,
    training_participation_report: Option<TrainingParticipationReportParameterSet>,
) -> Result<(EventResource, Option<MailResource>), CaptureApiError> {
    let recurrence_pattern = recurrence_pattern.to_multiline_string();

    let (duration_secs, ends_at_dt, ends_at_tz) =
        parse_event_dt_params(is_all_day, starts_at, ends_at, &recurrence_pattern)?;

    let room = inventory
        .create_room(NewRoom {
            created_by: current_user.id,
            password,
            waiting_room,
            tenant_id: current_user.tenant_id,
            e2e_encryption,
        })
        .await?;

    let sip_config = inventory
        .create_room_sip_config(NewRoomSipConfig::new(room.id, false))
        .await?;

    let event = inventory
        .create_event(NewEvent {
            title,
            description,
            room: room.id,
            created_by: current_user.id,
            updated_by: current_user.id,
            is_adhoc,
            show_meeting_details,
            tenant_id: current_user.tenant_id,
            date: Some(NewEventDate {
                is_all_day,
                starts_at: starts_at.to_datetime_tz(),
                starts_at_tz: starts_at.timezone,
                ends_at: ends_at_dt,
                ends_at_tz,
                recurrence: duration_secs.zip(recurrence_pattern).map(
                    |(duration_secs, recurrence_pattern)| NewEventRecurrence {
                        duration_secs,
                        recurrence_pattern,
                    },
                ),
            }),
        })
        .await?;

    let streaming_targets =
        store_event_streaming_targets(inventory, event.id, streaming_targets).await?;

    let training_participation_report = if let Some(parameters) = training_participation_report {
        store_training_participation_report(inventory, event.id, parameters).await?
    } else {
        None
    };

    let suppress_email_notification = is_adhoc || query.suppress_email_notification;

    let date = event
        .date()
        .map(|date| EventResourceDateKind::TimeDependent {
            is_time_independent: TimeDependentMarker,
            date: event
                .recurrence()
                .and_then(|recurrence| {
                    recurrence
                        .recurrence_pattern
                        .parse::<RecurrencePattern>()
                        .ok()
                })
                .map(|recurrence_pattern| EventResourceDate::Recurring {
                    is_all_day: date.is_all_day,
                    starts_at: DateTimeTz {
                        datetime: date.starts_at.into(),
                        timezone: date.starts_at_tz,
                    },
                    ends_at: DateTimeTz {
                        datetime: date.ends_at.into(),
                        timezone: date.ends_at_tz,
                    },
                    recurrence_pattern,
                })
                .unwrap_or_else(|| EventResourceDate::Single {
                    starts_at: DateTimeTz {
                        datetime: date.starts_at.into(),
                        timezone: date.starts_at_tz,
                    },
                    ends_at: DateTimeTz {
                        datetime: date.ends_at.into(),
                        timezone: date.ends_at_tz,
                    },
                    is_all_day: date.is_all_day,
                }),
        })
        .unwrap_or(EventResourceDateKind::TIME_INDEPENDENT);

    let event_resource = EventResource {
        id: event.id,
        title: event.title.clone(),
        description: event.description.clone(),
        room: EventRoomInfo::from_room(settings, &room, Some(&sip_config), user_tariff),
        invitees_truncated: false,
        invitees: vec![],
        created_by: current_user.to_public_user_profile(settings),
        created_at: event.created_at,
        updated_by: current_user.to_public_user_profile(settings),
        updated_at: event.updated_at,
        invite_status: EventInviteStatus::Accepted,
        is_favorite: false,
        can_edit: true, // Just created by the current user.
        is_adhoc,
        shared_folder: None,
        streaming_targets,
        show_meeting_details,
        training_participation_report,
        date,
    };

    let mail_resource = (!suppress_email_notification).then(|| MailResource {
        current_user: current_user.clone(),
        event,
        room,
        sip_config: Some(sip_config),
    });

    Ok((event_resource, mail_resource))
}

pub(crate) struct CancellationNotificationValues {
    pub tenant: Tenant,
    pub created_by: User,
    pub event: Event,
    pub room: Room,
    pub sip_config: Option<RoomSipConfig>,
    pub users_to_notify: Vec<MailRecipient>,
    pub shared_folder: Option<SharedFolder>,
    pub streaming_targets: Vec<RoomStreamingTarget>,
}

/// Part of `DELETE /events/{event_id}` (see [`delete_event`]).
///
/// Notify invited users about the event deletion.
pub(crate) async fn notify_invitees_about_delete(
    settings: &Settings,
    room_tariff: &TariffResource,
    notification_values: CancellationNotificationValues,
    mail_service: &MailService,
    user_search_client: &Option<KeycloakAdminClient>,
) {
    // Don't send mails for past events.
    if let Some(date) = notification_values.event.date()
        && date.ends_at < Utc::now().into()
    {
        return;
    }

    for user in notification_values.users_to_notify {
        let invited_user = enrich_from_optional_user_search(
            settings,
            user,
            &notification_values.tenant,
            user_search_client,
        )
        .await;

        if let Err(e) = mail_service
            .send_event_cancellation(
                settings,
                notification_values.created_by.clone(),
                notification_values.event.clone(),
                notification_values.room.clone(),
                room_tariff,
                notification_values.sip_config.clone(),
                invited_user,
                notification_values.shared_folder.clone(),
                notification_values.streaming_targets.clone(),
            )
            .await
        {
            log::error!(
                "Failed to send event cancellation with MailService, {}",
                Report::from_error(e)
            );
        }
    }
}

async fn get_invitees_for_event(
    settings: &Settings,
    inventory: &mut dyn Inventory,
    event_id: EventId,
    invitees_max: Option<PageSize>,
) -> opentalk_inventory::Result<(Vec<EventInvitee>, bool)> {
    let Some(invitees_max) = invitees_max else {
        return Ok((vec![], true));
    };

    // Get regular invitees up to the maximum invitee count specified.
    let (invites_with_user, total_invites_count) = inventory
        .get_event_invites_paginated(event_id, invitees_max, Page::DEFAULT, None)
        .await?;

    let mut invitees: Vec<EventInvitee> = invites_with_user
        .into_iter()
        .map(|(invite, user)| EventInvitee::from_invite_with_user(invite, user, settings))
        .collect();

    let loaded_invites_count = ItemCount::try_from(invitees.len())
        .expect("looks like we got more items than can be represented in the ItemCount type");
    let mut invitees_truncated = total_invites_count > loaded_invites_count;

    if loaded_invites_count == invitees_max {
        return Ok((invitees, invitees_truncated));
    }

    // Now add email invitees until the maximum total invitee count specified is reached.
    let invitees_max = if loaded_invites_count.is_zero() {
        invitees_max
    } else {
        let loaded_invites_page_size = PageSize::try_from(i64::from(loaded_invites_count))
            .expect("Attempted to load a PageSize from an invalid value, this is likely a bug");
        invitees_max.saturating_sub(loaded_invites_page_size)
    };

    let (email_invites, total_email_invites_count) = inventory
        .get_event_email_invites_paginated(event_id, invitees_max, Page::DEFAULT)
        .await?;

    let email_invitees: Vec<EventInvitee> = email_invites
        .into_iter()
        .map(|invite| EventInvitee::from_email_invite(invite, settings))
        .collect();

    let loaded_email_invites_count = ItemCount::try_from(email_invitees.len())
        .expect("looks like we got more items than can be represented in the ItemCount type");
    invitees_truncated =
        invitees_truncated || (total_email_invites_count > loaded_email_invites_count);

    invitees.extend(email_invitees);

    Ok((invitees, invitees_truncated))
}

fn verify_exception_dt_params(
    is_all_day: bool,
    starts_at: DateTimeTz,
    ends_at: DateTimeTz,
) -> Result<(), ApiError> {
    parse_event_dt_params(is_all_day, starts_at, ends_at, &None).map(|_| ())
}

/// parse the given event dt params
///
/// checks that the given params are valid to be put in the database
///
/// That means that:
/// - starts_at >= ends_at
/// - if is_all_day: starts_at & ends_at have their time part at 00:00
/// - bounded recurrence_pattern yields at least one result
///
/// returns the duration of the event if its recurring
/// and the appropriate ends_at datetime and timezone
fn parse_event_dt_params(
    is_all_day: bool,
    starts_at: DateTimeTz,
    ends_at: DateTimeTz,
    recurrence_pattern: &Option<String>,
) -> Result<(Option<i32>, DateTime<Tz>, TimeZone), ApiError> {
    const CODE_INVALID_EVENT: &str = "invalid_event";

    let starts_at_dt = starts_at.to_datetime_tz();
    let ends_at_dt = ends_at.to_datetime_tz();

    let duration_secs = (ends_at_dt - starts_at_dt).num_seconds();

    if duration_secs < 0 {
        return Err(ApiError::unprocessable_entity()
            .with_code(CODE_INVALID_EVENT)
            .with_message("ends_at must not be before starts_at"));
    }

    if is_all_day {
        let zero = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        if starts_at.datetime.time() != zero || ends_at.datetime.time() != zero {
            return Err(ApiError::unprocessable_entity()
                .with_code(CODE_INVALID_EVENT)
                .with_message(
                    "is_all_day requires starts_at/ends_at to be set at the start of the day",
                ));
        }
    }

    if let Some(recurrence_pattern) = &recurrence_pattern {
        let starts_at_tz = starts_at.timezone;
        let starts_at_fmt = starts_at.datetime.format(LOCAL_DT_FORMAT);

        let rrule_set =
            format!("DTSTART;TZID={starts_at_tz}:{starts_at_fmt}\n{recurrence_pattern}");
        let rrule_set = match rrule_set.parse::<RRuleSet>() {
            Ok(rrule) => rrule,
            Err(e) => {
                log::warn!("failed to parse rrule {}", Report::from_error(e));
                return Err(ApiError::unprocessable_entity()
                    .with_code(CODE_INVALID_EVENT)
                    .with_message("Invalid recurrence pattern"));
            }
        };

        if rrule_set
            .get_rrule()
            .iter()
            .any(|rrule| rrule.get_freq() > Frequency::Daily)
        {
            return Err(ApiError::unprocessable_entity()
                .with_code(CODE_INVALID_EVENT)
                .with_message("Frequencies below 'DAILY' are not supported"));
        }

        // Figure out ends_at timestamp
        // Check if all RRULEs are reasonably bounded in how far they go
        let is_bounded = rrule_set.get_rrule().iter().all(|rrule| {
            if let Some(count) = rrule.get_count()
                && count < 1000
            {
                return true;
            }

            if let Some(until) = rrule.get_until()
                && (until.naive_utc() - starts_at.datetime.naive_utc()).num_days()
                    <= ONE_HUNDRED_YEARS_IN_DAYS as i64
            {
                return true;
            }

            false
        });

        let dt_of_last_occurrence = if is_bounded {
            // For bounded RRULEs calculate the date of the last occurrence
            // Still limiting the iterations - just in case
            rrule_set
                .into_iter()
                .take(ONE_HUNDRED_YEARS_IN_DAYS)
                .last()
                .ok_or_else(|| {
                    ApiError::unprocessable_entity()
                        .with_code(CODE_INVALID_EVENT)
                        .with_message("recurrence_pattern does not yield any dates")
                })?
                .with_timezone(ends_at.timezone.as_ref())
        } else {
            // For RRULEs for which calculating the last occurrence might take too
            // long, as they run forever or into the very far future, just take a
            // date 100 years from the start date (or if invalid fall back to the chrono MAX DATE)
            starts_at
                .datetime
                .with_year(ends_at_dt.year() + 100)
                .unwrap_or(DateTime::<Utc>::MAX_UTC)
                .with_timezone(ends_at.timezone.as_ref())
        };

        Ok((
            Some(duration_secs as i32),
            dt_of_last_occurrence,
            ends_at.timezone,
        ))
    } else {
        Ok((None, ends_at.to_datetime_tz(), ends_at.timezone))
    }
}

/// Helper trait to to reduce boilerplate in the single route handlers
///
/// Bundles multiple resources into groups.
pub trait EventPoliciesBuilderExt {
    /// Adds permissions for reading events
    fn event_read_access(self, event_id: EventId) -> Self;
    /// Adds permissions for writing events
    fn event_write_access(self, event_id: EventId) -> Self;

    /// Adds permissions for event invites
    fn event_invite_invitee_access(self, event_id: EventId) -> Self;
}

impl<T> EventPoliciesBuilderExt for PoliciesBuilder<GrantingAccess<T>>
where
    T: IsSubject + Clone,
{
    /// GET access to the event and related endpoints.
    /// PUT and DELETE to the event_favorites endpoint.
    fn event_read_access(self, event_id: EventId) -> Self {
        self.add_resource(event_id.resource_id(), [AccessMethod::Get])
            .add_resource(
                event_id.resource_id().with_suffix("/instances"),
                [AccessMethod::Get],
            )
            .add_resource(
                event_id.resource_id().with_suffix("/instances/*"),
                [AccessMethod::Get],
            )
            .add_resource(
                event_id.resource_id().with_suffix("/invites"),
                [AccessMethod::Get],
            )
            .add_resource(
                event_id.resource_id().with_suffix("/shared_folder"),
                [AccessMethod::Get],
            )
            .add_resource(
                format!("/users/me/event_favorites/{event_id}"),
                [AccessMethod::Put, AccessMethod::Delete],
            )
    }

    /// PATCH and DELETE to the event
    /// POST to reschedule and invites of the event
    /// PATCH to instances
    /// DELETE to invites
    fn event_write_access(self, event_id: EventId) -> Self {
        self.add_resource(
            event_id.resource_id(),
            [AccessMethod::Patch, AccessMethod::Delete],
        )
        .add_resource(
            event_id.resource_id().with_suffix("/reschedule"),
            [AccessMethod::Post],
        )
        .add_resource(
            event_id.resource_id().with_suffix("/instances/*"),
            [AccessMethod::Patch],
        )
        .add_resource(
            event_id.resource_id().with_suffix("/invites"),
            [AccessMethod::Post],
        )
        .add_resource(
            event_id.resource_id().with_suffix("/invites/*"),
            [AccessMethod::Patch, AccessMethod::Delete],
        )
        .add_resource(
            event_id.resource_id().with_suffix("/shared_folder"),
            [AccessMethod::Put, AccessMethod::Delete],
        )
    }

    /// PATCH and DELETE to event invite
    fn event_invite_invitee_access(self, event_id: EventId) -> Self {
        self.add_resource(
            format!("/events/{event_id}/invite"),
            [AccessMethod::Patch, AccessMethod::Delete],
        )
    }
}

fn compare_inventory_events_or_exceptions(
    item_1: &InternalEventOrException,
    item_2: &InternalEventOrException,
) -> Ordering {
    let (starts_at_1, created_at_1, event_id_1, instance_id_1) = match item_1 {
        InternalEventOrException::Event((
            event,
            _invite,
            _room,
            _sip_config,
            _is_favorite,
            _shared_folder,
            _tariff,
            _training_participation_report_parameter_set,
        )) => (
            event.date().map(|date| date.starts_at),
            event.created_at,
            event.id,
            None,
        ),
        InternalEventOrException::Exception((event_exception, _event)) => (
            event_exception.starts_at,
            event_exception.created_at,
            event_exception.event_id,
            Some(event_exception.exception_date),
        ),
    };

    let (starts_at_2, created_at_2, event_id_2, instance_id_2) = match item_2 {
        InternalEventOrException::Event((
            event,
            _invite,
            _room,
            _sip_config,
            _is_favorite,
            _shared_folder,
            _tariff,
            _training_participation_report_parameter_set,
        )) => (
            event.date().map(|date| date.starts_at),
            event.created_at,
            event.id,
            None,
        ),
        InternalEventOrException::Exception((event_exception, _event)) => (
            event_exception.starts_at,
            event_exception.created_at,
            event_exception.event_id,
            Some(event_exception.exception_date),
        ),
    };

    // This comparison requires that the streams are properly sorted with NULLs (i.e. `None` values) first.
    match (starts_at_1, starts_at_2) {
        (None, _) => Ordering::Less,
        (_, None) => Ordering::Greater,
        (Some(starts_at_1), Some(starts_at_2)) => starts_at_1.cmp(&starts_at_2),
    }
    .then(created_at_1.cmp(&created_at_2))
    .then(event_id_1.cmp(&event_id_2))
    .then(match (instance_id_1, instance_id_2) {
        (None, _) => Ordering::Less,
        (_, None) => Ordering::Greater,
        (Some(instance_id_1), Some(instance_id_2)) => instance_id_1.cmp(&instance_id_2),
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use opentalk_types_common::{
        events::invites::InviteRole,
        rooms::RoomId,
        time::{TimeZone, Timestamp},
        training_participation_report::TimeRange,
        users::{UserId, UserInfo},
    };
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    #[test]
    fn rrulset_parse_works_as_used_in_this_crate() {
        assert!(
            "DTSTART;TZID=Europe/Vienna:20230723T080000\nRRULE:FREQ=DAILY;UNTIL=20240119T100000Z"
                .parse::<RRuleSet>()
                .is_ok()
        );
        // Note the semicolon before the `\n`
        assert!(
            "DTSTART;TZID=Europe/Vienna:20230723T080000;\nRRULE:FREQ=DAILY;UNTIL=20240119T100000Z"
                .parse::<RRuleSet>()
                .is_err()
        );
    }

    #[test]
    fn event_resource_serialize() {
        let unix_epoch: Timestamp = SystemTime::UNIX_EPOCH.into();

        let user_profile = PublicUserProfile {
            id: UserId::nil(),
            email: "test@example.org".into(),
            user_info: UserInfo {
                title: "".parse().expect("valid user title"),
                firstname: "Test".into(),
                lastname: "Test".into(),
                display_name: "Tester".parse().expect("valid display name"),
                avatar_url: "https://example.org/avatar".into(),
            },
        };

        let event_resource = EventResource {
            id: EventId::nil(),
            created_by: user_profile.clone(),
            created_at: unix_epoch,
            updated_by: user_profile.clone(),
            updated_at: unix_epoch,
            title: "Event title".parse().expect("valid event title"),
            description: "Event description"
                .parse()
                .expect("valid event description"),
            room: EventRoomInfo {
                id: RoomId::nil(),
                password: None,
                waiting_room: false,
                e2e_encryption: false,
                call_in: None,
            },
            invitees_truncated: false,
            invitees: vec![EventInvitee {
                profile: EventInviteeProfile::Registered(PublicInviteUserProfile {
                    user_profile,
                    role: InviteRole::Moderator,
                }),
                status: EventInviteStatus::Accepted,
            }],
            date: EventResourceDateKind::TimeDependent {
                is_time_independent: TimeDependentMarker,
                date: EventResourceDate::Recurring {
                    is_all_day: false,
                    starts_at: DateTimeTz {
                        datetime: *unix_epoch,
                        timezone: TimeZone::from(Tz::Europe__Berlin),
                    },
                    ends_at: DateTimeTz {
                        datetime: *unix_epoch,
                        timezone: TimeZone::from(Tz::Europe__Berlin),
                    },
                    recurrence_pattern: RecurrencePattern::default(),
                },
            },
            invite_status: EventInviteStatus::Accepted,
            is_favorite: false,
            can_edit: true,
            is_adhoc: false,
            shared_folder: None,
            streaming_targets: Vec::new(),
            show_meeting_details: true,
            training_participation_report: Some(TrainingParticipationReportParameterSet {
                initial_checkpoint_delay: TimeRange::new_with_clamped_durations(
                    Duration::from_secs(100),
                    Duration::from_secs(200),
                ),
                checkpoint_interval: TimeRange::new_with_clamped_durations(
                    Duration::from_secs(300),
                    Duration::from_secs(400),
                ),
            }),
        };

        assert_eq!(
            json!(event_resource),
            json!({
                    "id": "00000000-0000-0000-0000-000000000000",
                    "created_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "created_at": "1970-01-01T00:00:00Z",
                    "updated_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "updated_at": "1970-01-01T00:00:00Z",
                    "title": "Event title",
                    "description": "Event description",
                    "room": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "waiting_room": false,
                        "e2e_encryption": false
                    },
                    "invitees_truncated": false,
                    "invitees": [
                        {
                            "profile": {
                                "kind": "registered",
                                "id": "00000000-0000-0000-0000-000000000000",
                                "email": "test@example.org",
                                "title": "",
                                "firstname": "Test",
                                "lastname": "Test",
                                "display_name": "Tester",
                                "avatar_url": "https://example.org/avatar",
                                "role": "moderator"
                            },
                            "status": "accepted"
                        }
                    ],
                    "is_time_independent": false,
                    "is_all_day": false,
                    "starts_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "ends_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "type": "recurring",
                    "invite_status": "accepted",
                    "is_favorite": false,
                    "can_edit": true,
                    "is_adhoc": false,
                    "show_meeting_details": true,
                    "training_participation_report": {
                        "initial_checkpoint_delay": {
                            "after": 100,
                            "within": 200,
                        },
                        "checkpoint_interval": {
                            "after": 300,
                            "within": 400,
                        },
                    },
                }
            )
        );
    }

    #[test]
    fn event_resource_time_independent_serialize() {
        let unix_epoch: Timestamp = SystemTime::UNIX_EPOCH.into();

        let user_profile = PublicUserProfile {
            id: UserId::nil(),
            email: "test@example.org".into(),
            user_info: UserInfo {
                title: "".parse().expect("valid user title"),
                firstname: "Test".into(),
                lastname: "Test".into(),
                display_name: "Tester".parse().expect("valid display name"),
                avatar_url: "https://example.org/avatar".into(),
            },
        };

        let event_resource = EventResource {
            id: EventId::nil(),
            created_by: user_profile.clone(),
            created_at: unix_epoch,
            updated_by: user_profile.clone(),
            updated_at: unix_epoch,
            title: "Event title".parse().expect("valid event title"),
            description: "Event description"
                .parse()
                .expect("valid event description"),
            room: EventRoomInfo {
                id: RoomId::nil(),
                password: None,
                waiting_room: false,
                e2e_encryption: false,
                call_in: Some(CallInInfo {
                    tel: "030123456".into(),
                    uri: None,
                    id: "1234567890".into(),
                    password: "1234567890".into(),
                }),
            },
            invitees_truncated: false,
            invitees: vec![EventInvitee {
                profile: EventInviteeProfile::Registered(PublicInviteUserProfile {
                    user_profile,
                    role: InviteRole::User,
                }),
                status: EventInviteStatus::Accepted,
            }],
            date: EventResourceDateKind::TIME_INDEPENDENT,
            invite_status: EventInviteStatus::Accepted,
            is_favorite: true,
            can_edit: false,
            is_adhoc: false,
            shared_folder: None,
            streaming_targets: Vec::new(),
            show_meeting_details: false,
            training_participation_report: Some(TrainingParticipationReportParameterSet {
                initial_checkpoint_delay: TimeRange::new_with_clamped_durations(
                    Duration::from_secs(100),
                    Duration::from_secs(200),
                ),
                checkpoint_interval: TimeRange::new_with_clamped_durations(
                    Duration::from_secs(300),
                    Duration::from_secs(400),
                ),
            }),
        };

        assert_eq!(
            json!(event_resource),
            json!({
                    "id": "00000000-0000-0000-0000-000000000000",
                    "created_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "created_at": "1970-01-01T00:00:00Z",
                    "updated_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "updated_at": "1970-01-01T00:00:00Z",
                    "title": "Event title",
                    "description": "Event description",
                    "room": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "waiting_room": false,
                        "e2e_encryption": false,
                        "call_in": {
                            "tel": "030123456",
                            "id": "1234567890",
                            "password": "1234567890",
                        }
                    },
                    "invitees_truncated": false,
                    "invitees": [
                        {
                            "profile": {
                                "kind": "registered",
                                "id": "00000000-0000-0000-0000-000000000000",
                                "email": "test@example.org",
                                "title": "",
                                "firstname": "Test",
                                "lastname": "Test",
                                "display_name": "Tester",
                                "avatar_url": "https://example.org/avatar",
                                "role": "user"
                            },
                            "status": "accepted"
                        }
                    ],
                    "is_time_independent": true,
                    "type": "single",
                    "invite_status": "accepted",
                    "is_favorite": true,
                    "can_edit": false,
                    "is_adhoc": false,
                    "show_meeting_details": false,
                    "training_participation_report": {
                        "initial_checkpoint_delay": {
                            "after": 100,
                            "within": 200,
                        },
                        "checkpoint_interval": {
                            "after": 300,
                            "within": 400,
                        },
                    },
                }
            )
        );
    }

    #[test]
    fn event_exception_serialize() {
        let unix_epoch: Timestamp = SystemTime::UNIX_EPOCH.into();
        let instance_id = unix_epoch.into();
        let event_id = EventId::nil();
        let user_profile = PublicUserProfile {
            id: UserId::nil(),
            email: "test@example.org".into(),
            user_info: UserInfo {
                title: "".parse().expect("valid user title"),
                firstname: "Test".into(),
                lastname: "Test".into(),
                display_name: "Tester".parse().expect("valid display name"),
                avatar_url: "https://example.org/avatar".into(),
            },
        };

        let instance = EventExceptionResource {
            id: EventAndInstanceId(event_id, instance_id),
            recurring_event_id: event_id,
            instance_id,
            created_by: user_profile.clone(),
            created_at: unix_epoch,
            updated_by: user_profile,
            updated_at: unix_epoch,
            title: Some("Instance title".parse().expect("valid event title")),
            description: Some(
                "Instance description"
                    .parse()
                    .expect("valid event description"),
            ),
            is_all_day: Some(false),
            starts_at: Some(DateTimeTz {
                datetime: *unix_epoch,
                timezone: TimeZone::from(Tz::Europe__Berlin),
            }),
            ends_at: Some(DateTimeTz {
                datetime: *unix_epoch,
                timezone: TimeZone::from(Tz::Europe__Berlin),
            }),
            original_starts_at: DateTimeTz {
                datetime: *unix_epoch,
                timezone: TimeZone::from(Tz::Europe__Berlin),
            },
            type_: ExceptionMarker::Exception,
            status: EventStatus::Ok,
            can_edit: false,
        };

        assert_eq!(
            json!(instance),
            json! ({
                    "id": "00000000-0000-0000-0000-000000000000_19700101T000000Z",
                    "recurring_event_id": "00000000-0000-0000-0000-000000000000",
                    "instance_id": "19700101T000000Z",
                    "created_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "created_at": "1970-01-01T00:00:00Z",
                    "updated_by": {
                        "id": "00000000-0000-0000-0000-000000000000",
                        "email": "test@example.org",
                        "title": "",
                        "firstname": "Test",
                        "lastname": "Test",
                        "display_name": "Tester",
                        "avatar_url": "https://example.org/avatar"
                    },
                    "updated_at": "1970-01-01T00:00:00Z",
                    "title": "Instance title",
                    "description": "Instance description",
                    "is_all_day": false,
                    "starts_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "ends_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "original_starts_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "type": "exception",
                    "status": "ok",
                    "can_edit": false,
                }
            )
        );
    }
}
