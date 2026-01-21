// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Handles event instances

use chrono::{DateTime, Duration, Utc};
use kustos::policies_builder::PoliciesBuilder;
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_utils::{
    CaptureApiError,
    event::{EventDateExt, EventExt},
};
use opentalk_inventory::{
    Event, EventDate, EventException, EventExceptionKind, NewEventException, UpdateEventException,
};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{
        EventAndInstanceId, EventInstance, EventInstancePath, EventInstanceQuery, EventInvitee,
        EventOrInstance, EventResourceDate, EventResourceDateKind, EventRoomInfo, EventStatus,
        EventType, GetEventInstanceResponseBody, GetEventInstancesCursorData,
        GetEventInstancesQuery, GetEventInstancesResponseBody, GetEventsAndInstancesQuery,
        GetEventsCursorData, GetEventsQuery, InstanceId, PatchEventInstanceBody,
    },
    pagination::Cursor,
};
use opentalk_types_common::{
    events::{EventId, invites::EventInviteStatus},
    pagination::{Page, PageSize},
    shared_folders::SharedFolder,
    time::{DateTimeTz, TimeZone, Timestamp},
    training_participation_report::TrainingParticipationReportParameterSet,
    users::Language,
};

use crate::{
    ControllerBackend,
    controller_backend::{
        RoomsPoliciesBuilderExt,
        events::{DateTimeTzFromInventory, EventRoomInfoExt, ONE_HUNDRED_YEARS_IN_DAYS},
    },
    events::{
        enrich_invitees_from_optional_user_search, get_invited_mail_recipients_for_event,
        notifications::{UpdateNotificationValues, notify_invitees_about_update},
        shared_folder_for_user,
    },
    user_profiles::{GetUserProfilesBatched, UserProfilesBatch},
};

const ONE_HUNDRED_YEARS: Duration = Duration::days(ONE_HUNDRED_YEARS_IN_DAYS as i64);

impl ControllerBackend {
    pub(crate) async fn get_events_and_instances(
        &self,
        current_user: RequestUser,
        query: GetEventsAndInstancesQuery,
    ) -> Result<(Vec<EventOrInstance>, Option<String>, Option<String>), CaptureApiError> {
        let after = query.after.map(|after| {
            Cursor(GetEventsCursorData {
                event_id: after.event_id,
                event_created_at: after.event_created_at,
                event_starts_at: after.event_starts_at,
                instance_id: None,
            })
        });

        let events_query = GetEventsQuery {
            time_min: query.time_min,
            time_max: query.time_max,
            created_before: query.created_before,
            created_after: query.created_after,
            invitees_max: query.invitees_max,
            favorites: query.favorites,
            invite_status: query.invite_status,
            per_page: query.per_page,
            after,
            adhoc: query.adhoc,
            time_independent: query.time_independent,
        };

        let (event_resources, before, after) = self
            .get_events_internal(current_user.clone(), events_query, false)
            .await?;

        let mut event_or_instance_resources: Vec<EventOrInstance> = vec![];

        // TODO: Incorporate instances into overall paging, all sorted by time? See #1088
        for (event_resource, _event_exception_resources) in event_resources {
            if query.instances_max.is_zero() {
                event_or_instance_resources.push(EventOrInstance::Event(event_resource));
                continue;
            };

            // A event that is not time-dependent can't be recurring, hence the
            // early return.
            let EventResourceDateKind::TimeDependent {
                is_time_independent: _,
                ref date,
            } = event_resource.date
            else {
                event_or_instance_resources.push(EventOrInstance::Event(event_resource));
                continue;
            };

            let EventResourceDate::Single { .. } = date else {
                event_or_instance_resources.push(EventOrInstance::Event(event_resource));
                continue;
            };

            let instances_query = GetEventInstancesQuery {
                invitees_max: query.invitees_max,
                time_min: query.time_min,
                time_max: query.time_max,
                per_page: Some(PageSize::from_i64_clamped(query.instances_max.into())),
                after: None,
            };

            // TODO: Optimize to get instances for a list of events (not just a single event) with less DB roundtrips? See #1088
            let instances_response = self
                .get_event_instances(&current_user, event_resource.id, instances_query)
                .await?;

            for instance_resource in instances_response.0.0 {
                event_or_instance_resources.push(EventOrInstance::Instance(instance_resource));
            }
        }

        Ok((event_or_instance_resources, before, after))
    }

    pub(crate) async fn get_event_instances(
        &self,
        current_user: &RequestUser,
        event_id: EventId,
        GetEventInstancesQuery {
            invitees_max,
            time_min,
            time_max,
            per_page,
            after,
        }: GetEventInstancesQuery,
    ) -> Result<
        (
            GetEventInstancesResponseBody,
            Option<String>,
            Option<String>,
        ),
        CaptureApiError,
    > {
        let settings = self.settings_provider.get();

        let per_page = per_page
            .unwrap_or(PageSize::from_i64_clamped(30))
            .clamp(PageSize::ONE, PageSize::from_i64_clamped(100));
        let page = after.map(|c| c.page).unwrap_or(Page::ONE).max(Page::ONE);

        let items_per_page = per_page.into();
        let page_index = page.as_zero_based_usize();
        let offset = items_per_page * page_index;

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let (
            event,
            invite,
            room,
            sip_config,
            is_favorite,
            shared_folder,
            tariff,
            training_participation_report_parameter_set,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;

        let (invitees, invitees_truncated) =
            super::get_invitees_for_event(&settings, inventory.as_mut(), event.id, invitees_max)
                .await?;

        let invite_status = invite
            .map(|inv| inv.status)
            .unwrap_or(EventInviteStatus::Accepted);

        let Some(rruleset) = event.to_rruleset()? else {
            return Err(ApiError::not_found().into());
        };

        const MONTHS_PER_YEAR: u32 = 12;

        // limit of how far into the future we calculate instances
        let max_dt = Utc::now()
            .with_timezone(&rruleset.get_dt_start().timezone())
            .checked_add_months(chrono::Months::new(40 * MONTHS_PER_YEAR))
            .expect("Could not add required duration");

        let mut iter: Box<dyn Iterator<Item = DateTime<rrule::Tz>>> =
            Box::new(rruleset.into_iter().skip_while(move |&dt| dt > max_dt));

        if let Some(time_min) = time_min {
            iter = Box::new(iter.skip_while(move |&dt| dt <= *time_min));
        }

        if let Some(time_max) = time_max {
            iter = Box::new(iter.skip_while(move |&dt| dt >= *time_max));
        }

        let datetimes: Vec<Timestamp> = iter
            .skip(offset)
            .take(items_per_page)
            .map(|dt| dt.with_timezone(&Utc).into())
            .collect();

        let exceptions = inventory.get_event_exceptions(event_id, &datetimes).await?;

        let users = GetUserProfilesBatched::new()
            .add(&event)
            .add(&exceptions)
            .fetch(&settings, inventory.as_mut())
            .await?;

        let training_participation_report = training_participation_report_parameter_set
            .map(TrainingParticipationReportParameterSet::from);

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        drop(inventory);

        let tariff = self.build_tariff_resource(&tariff)?;

        let room = EventRoomInfo::from_room(&settings, &room, sip_config.as_ref(), &tariff);

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let mut exceptions = exceptions.into_iter().peekable();

        let mut instances = vec![];

        for datetime in datetimes {
            let exception = exceptions.next_if(|exception| exception.exception_date == datetime);

            let instance = create_event_instance(
                &users,
                event.clone(),
                invite_status,
                is_favorite,
                exception,
                room.clone(),
                datetime.into(),
                invitees.clone(),
                invitees_truncated,
                can_edit,
                shared_folder.clone(),
                training_participation_report.clone(),
            )?;

            instances.push(instance);
        }

        let next_cursor = if !instances.is_empty() {
            Some(
                Cursor(GetEventInstancesCursorData {
                    page: page.saturating_next(),
                })
                .to_base64(),
            )
        } else {
            None
        };

        let instances_data = GetPaginatedEventInstancesData {
            instances,
            before: None,
            after: next_cursor,
        };

        // Enrich the invitees for the first instance only and reuse them as all instances have the same invitees.
        let event_instances = if let Some(instance) = instances_data.instances.first() {
            let enriched_invitees = enrich_invitees_from_optional_user_search(
                &settings,
                &self.user_search_client,
                &current_tenant,
                instance.invitees.clone(),
            )
            .await;

            instances_data
                .instances
                .into_iter()
                .map(|instance| EventInstance {
                    invitees: enriched_invitees.clone(),
                    ..instance
                })
                .collect()
        } else {
            instances_data.instances
        };

        Ok((
            GetEventInstancesResponseBody(event_instances),
            instances_data.before,
            instances_data.after,
        ))
    }

    pub(crate) async fn get_event_instance(
        &self,
        current_user: &RequestUser,
        EventInstancePath {
            event_id,
            instance_id,
        }: EventInstancePath,
        query: EventInstanceQuery,
    ) -> Result<GetEventInstanceResponseBody, CaptureApiError> {
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
            training_participation_report_parameter_set,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;

        let Some(event_date) = event.date() else {
            return Err(ApiError::not_found().into());
        };

        let instance_date = DateTime::from(instance_id).with_timezone(&event_date.starts_at_tz);

        verify_instance_date(event_date, instance_date)?;

        let (invitees, invitees_truncated) = super::get_invitees_for_event(
            &settings,
            inventory.as_mut(),
            event_id,
            query.invitees_max,
        )
        .await?;

        let exception = inventory
            .get_event_exception(event_id, instance_id.into())
            .await?;

        let users = GetUserProfilesBatched::new()
            .add(&event)
            .add(&exception)
            .fetch(&settings, inventory.as_mut())
            .await?;

        let tariff = self.build_tariff_resource(&tariff)?;

        let room = EventRoomInfo::from_room(&settings, &room, sip_config.as_ref(), &tariff);

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let event_instance = create_event_instance(
            &users,
            event,
            invite
                .map(|inv| inv.status)
                .unwrap_or(EventInviteStatus::Accepted),
            is_favorite,
            exception,
            room,
            instance_id,
            invitees,
            invitees_truncated,
            can_edit,
            shared_folder,
            training_participation_report_parameter_set.map(Into::into),
        )?;

        let event_instance = EventInstance {
            invitees: enrich_invitees_from_optional_user_search(
                &settings,
                &self.user_search_client,
                &current_tenant,
                event_instance.invitees,
            )
            .await,
            ..event_instance
        };

        Ok(GetEventInstanceResponseBody(event_instance))
    }

    pub(crate) async fn patch_event_instance(
        &self,
        current_user: RequestUser,
        EventInstancePath {
            event_id,
            instance_id,
        }: EventInstancePath,
        EventInstanceQuery {
            invitees_max,
            suppress_email_notification,
        }: EventInstanceQuery,
        patch: PatchEventInstanceBody,
    ) -> Result<Option<EventInstance>, CaptureApiError> {
        if patch.is_empty() {
            return Ok(None);
        }

        let mut inventory = self.inventory_provider.get_inventory().await?;

        let settings = self.settings_provider.get();
        let default_user_language = Language(settings.defaults.user_language.clone());

        let (
            event,
            invite,
            room,
            sip_config,
            is_favorite,
            shared_folder,
            tariff,
            training_participation_report_parameter_set,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;

        let Some(event_date) = event.date() else {
            return Err(ApiError::not_found().into());
        };

        let instance_date = DateTime::from(instance_id).with_timezone(&event_date.starts_at_tz);

        verify_instance_date(event_date, instance_date)?;

        let is_all_day = patch.is_all_day.unwrap_or(event_date.is_all_day);

        let starts_at = patch.starts_at.unwrap_or(DateTimeTz {
            datetime: event_date.starts_at.into(),
            timezone: event_date.starts_at_tz,
        });

        let ends_at = patch.ends_at.unwrap_or_else(|| {
            let (ends_at, ends_at_tz) = event_date.ends_at_of_first_occurrence();
            DateTimeTz {
                datetime: ends_at.into(),
                timezone: ends_at_tz,
            }
        });

        super::verify_exception_dt_params(is_all_day, starts_at, ends_at)?;

        let exception = match inventory
            .get_event_exception(event_id, instance_id.into())
            .await?
        {
            Some(exception) => {
                inventory
                    .update_event_exception(
                        exception.id,
                        UpdateEventException {
                            kind: match patch.status {
                                Some(EventStatus::Ok) => Some(EventExceptionKind::Modified),
                                Some(EventStatus::Cancelled) => Some(EventExceptionKind::Cancelled),
                                None => None,
                            },
                            title: patch.title.map(Some),
                            description: patch.description.map(Some),
                            is_all_day: patch.is_all_day.map(Some),
                            starts_at: patch.starts_at.map(|dt| Some(dt.to_datetime_tz())),
                            starts_at_tz: patch.starts_at.map(|dt| Some(dt.timezone)),
                            ends_at: patch.ends_at.map(|dt| Some(dt.to_datetime_tz())),
                            ends_at_tz: patch.ends_at.map(|dt| Some(dt.timezone)),
                        },
                    )
                    .await?
            }
            None => {
                inventory
                    .create_event_exception(NewEventException {
                        event_id: event.id,
                        exception_date: instance_id.into(),
                        exception_date_tz: event_date.starts_at_tz,
                        created_by: current_user.id,
                        kind: match patch.status {
                            Some(EventStatus::Cancelled) => EventExceptionKind::Cancelled,
                            _ => EventExceptionKind::Modified,
                        },
                        title: patch.title,
                        description: patch.description,
                        is_all_day: patch.is_all_day,
                        starts_at: patch.starts_at.map(|dt| dt.to_datetime_tz()),
                        starts_at_tz: patch.starts_at.map(|dt| dt.timezone),
                        ends_at: patch.ends_at.map(|dt| dt.to_datetime_tz()),
                        ends_at_tz: patch.ends_at.map(|dt| dt.timezone),
                    })
                    .await?
            }
        };

        let (invitees, invitees_truncated) =
            super::get_invitees_for_event(&settings, inventory.as_mut(), event_id, invitees_max)
                .await?;

        let users = GetUserProfilesBatched::new()
            .add(&event)
            .add(&exception)
            .fetch(&settings, inventory.as_mut())
            .await?;

        let tariff = self.build_tariff_resource(&tariff)?;

        let event_room_info =
            EventRoomInfo::from_room(&settings, &room, sip_config.as_ref(), &tariff);

        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
        let current_user = inventory.get_user(current_user.id).await?;

        let can_edit = current_user.can_edit(&event);

        let shared_folder =
            shared_folder_for_user(shared_folder, event.created_by, current_user.id);

        let streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

        if !suppress_email_notification {
            let invited_users = get_invited_mail_recipients_for_event(
                inventory.as_mut(),
                event_id,
                default_user_language,
            )
            .await?;
            let invite_for_room = inventory
                .get_or_create_valid_invite_for_room(room.id, current_user.id)
                .await?;

            let created_by = if event.created_by == current_user.id {
                current_user
            } else {
                inventory.get_user(event.created_by).await?
            };

            // Add the access policy for the invite code, just in case it has been created by
            // the `Invite::get_first_for_room(…)` call above. That function is not able to
            // add the policy, because it has no access to the `RoomsPoliciesBuilderExt` trait.
            let policies = PoliciesBuilder::new()
                // Grant invitee access
                .grant_invite_access(invite_for_room.invite_code)
                .room_guest_read_access(room.id)
                .finish();
            self.authz.add_policies(policies).await?;

            if let Some(mail_service) = self.mail_service.as_ref() {
                let notification_values = UpdateNotificationValues {
                    tenant: current_tenant.clone(),
                    created_by,
                    event: event.clone(),
                    event_exception: Some(exception.clone()),
                    room,
                    sip_config,
                    users_to_notify: invited_users,
                    invite_for_room,
                };

                notify_invitees_about_update(
                    &settings,
                    &tariff,
                    notification_values,
                    mail_service,
                    &self.user_search_client,
                    None,
                    streaming_targets,
                )
                .await;
            }
        }

        drop(inventory);

        let event_instance = create_event_instance(
            &users,
            event,
            invite
                .map(|inv| inv.status)
                .unwrap_or(EventInviteStatus::Accepted),
            is_favorite,
            Some(exception),
            event_room_info,
            instance_id,
            invitees,
            invitees_truncated,
            can_edit,
            shared_folder,
            training_participation_report_parameter_set.map(Into::into),
        )?;

        let event_instance = EventInstance {
            invitees: enrich_invitees_from_optional_user_search(
                &settings,
                &self.user_search_client,
                &current_tenant,
                event_instance.invitees,
            )
            .await,
            ..event_instance
        };

        Ok(Some(event_instance))
    }
}

struct GetPaginatedEventInstancesData {
    instances: Vec<EventInstance>,
    before: Option<String>,
    after: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn create_event_instance(
    users: &UserProfilesBatch,
    mut event: Event,
    invite_status: EventInviteStatus,
    is_favorite: bool,
    exception: Option<EventException>,
    room: EventRoomInfo,
    instance_id: InstanceId,
    invitees: Vec<EventInvitee>,
    invitees_truncated: bool,
    can_edit: bool,
    shared_folder: Option<SharedFolder>,
    training_participation_report: Option<TrainingParticipationReportParameterSet>,
) -> opentalk_database::Result<EventInstance> {
    let instance_date = event
        .date()
        .expect("event instances can only be created for events with a date");

    let instance_duration_secs = event
        .duration_secs()
        .expect("event instances can only be created for recurring events");

    let mut instance_starts_at = instance_id.into();
    let mut instance_starts_at_tz = instance_date.starts_at_tz;
    let mut instance_ends_at = instance_id + Duration::seconds(instance_duration_secs as i64);
    let mut instance_ends_at_tz = instance_date.ends_at_tz;

    let is_all_day = instance_date.is_all_day;

    let mut status = EventStatus::Ok;

    if let Some(exception) = exception {
        match exception.kind {
            EventExceptionKind::Modified => {} // Do nothing for now
            EventExceptionKind::Cancelled => status = EventStatus::Cancelled,
        }

        event.updated_by = exception.created_by;
        event.updated_at = exception.created_at;

        patch(&mut event.title, exception.title);
        patch(&mut event.description, exception.description);
        patch(&mut instance_starts_at, exception.starts_at);
        patch(&mut instance_starts_at_tz, exception.starts_at_tz);
        patch(
            &mut instance_ends_at,
            exception.ends_at.map(InstanceId::from),
        );
        patch(&mut instance_ends_at_tz, exception.ends_at_tz);
    }

    let created_by = users.get(event.created_by);
    let updated_by = users.get(event.updated_by);

    Ok(EventInstance {
        id: EventAndInstanceId(event.id, instance_id),
        recurring_event_id: event.id,
        instance_id,
        created_by,
        created_at: event.created_at,
        updated_by,
        updated_at: event.updated_at,
        title: event.title,
        description: event.description,
        room,
        invitees_truncated,
        invitees,
        is_all_day,
        starts_at: DateTimeTz {
            datetime: instance_starts_at.into(),
            timezone: instance_starts_at_tz,
        },
        ends_at: DateTimeTz {
            datetime: instance_ends_at.into(),
            timezone: instance_ends_at_tz,
        },
        type_: EventType::Instance,
        status,
        invite_status,
        is_favorite,
        can_edit,
        shared_folder,
        training_participation_report,
    })
}

fn patch<T>(dst: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *dst = value;
    }
}

fn verify_instance_date(
    event_date: &EventDate,
    instance_date: DateTime<TimeZone>,
) -> Result<(), ApiError> {
    if instance_date > DateTime::from(event_date.starts_at) + ONE_HUNDRED_YEARS {
        // TODO(t.spamer): I would argue this is a
        // ApiError::bad_request().with_message("requested date exceeds maximum 100-year range").
        // Change in follow up MR.
        return Err(ApiError::not_found());
    }

    let Some(rruleset) = event_date.to_rruleset()? else {
        return Err(ApiError::not_found());
    };

    let has_instance = rruleset
        .into_iter()
        .take_while(|dt| dt <= &instance_date)
        .any(|x| x == instance_date);

    if !has_instance {
        return Err(ApiError::not_found());
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use chrono_tz::Tz;
    use opentalk_types_api_v1::{
        events::{EventInviteeProfile, PublicInviteUserProfile},
        users::PublicUserProfile,
    };
    use opentalk_types_common::{
        events::{EventId, invites::InviteRole},
        rooms::RoomId,
        time::{TimeZone, Timestamp},
        users::{UserId, UserInfo},
    };
    use serde_json::json;

    use super::*;

    #[test]
    fn event_instance_serialize() {
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

        let instance = EventInstance {
            id: EventAndInstanceId(event_id, instance_id),
            recurring_event_id: event_id,
            instance_id,
            created_by: user_profile.clone(),
            created_at: unix_epoch,
            updated_by: user_profile.clone(),
            updated_at: unix_epoch,
            title: "Instance title".parse().expect("valid event title"),
            description: "Instance description"
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
                    role: InviteRole::User,
                }),
                status: EventInviteStatus::Accepted,
            }],
            is_all_day: false,
            starts_at: DateTimeTz {
                datetime: *unix_epoch,
                timezone: TimeZone::from(Tz::Europe__Berlin),
            },
            ends_at: DateTimeTz {
                datetime: *unix_epoch,
                timezone: TimeZone::from(Tz::Europe__Berlin),
            },
            type_: EventType::Instance,
            status: EventStatus::Ok,
            invite_status: EventInviteStatus::Accepted,
            is_favorite: false,
            can_edit: false,
            shared_folder: None,
            training_participation_report: None,
        };

        assert_eq!(
            json!(instance),
            json!({
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
                                "role": "user"

                            },
                            "status": "accepted"
                        }
                    ],
                    "is_all_day": false,
                    "starts_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "ends_at": {
                        "datetime": "1970-01-01T00:00:00Z",
                        "timezone": "Europe/Berlin"
                    },
                    "type": "instance",
                    "status": "ok",
                    "invite_status": "accepted",
                    "is_favorite": false,
                    "can_edit": false,
                }
            )
        );
    }
}
