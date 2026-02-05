// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{collections::HashSet, sync::Arc};

use kustos::Authz;
use log::Log;
use opentalk_controller_settings::Settings;
use opentalk_controller_utils::deletion::{Deleter, user::UserDeleter};
use opentalk_inventory::{Inventory, InventoryProvider, UpdateEvent, UpdateRoomInvite};
use opentalk_log::{debug, info, warn};
use opentalk_signaling_core::{ExchangeHandle, ObjectStorage};
use opentalk_types_common::{events::EventId, rooms::RoomId, time::Timestamp, users::UserId};
use snafu::Report;

use crate::Error;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum DeleteSelector {
    DisabledBefore(Timestamp),
}

pub(crate) async fn perform_deletion(
    logger: &dyn Log,
    inventory_provider: Arc<dyn InventoryProvider>,
    authz: Authz,
    exchange_handle: ExchangeHandle,
    settings: &Settings,
    fail_on_shared_folder_deletion_error: bool,
    delete_selector: DeleteSelector,
) -> Result<(), Error> {
    let mut inventory = inventory_provider.get_inventory().await?;
    let object_storage = ObjectStorage::new(&settings.minio).await?;

    delete_users(
        logger,
        inventory.as_mut(),
        &authz,
        exchange_handle.clone(),
        settings,
        &object_storage,
        fail_on_shared_folder_deletion_error,
        delete_selector,
    )
    .await?;

    Ok(())
}

/// Identify and delete users according to the specified delete selector
#[allow(clippy::too_many_arguments)]
async fn delete_users(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    authz: &Authz,
    exchange_handle: ExchangeHandle,
    settings: &Settings,
    object_storage: &ObjectStorage,
    fail_on_shared_folder_deletion_error: bool,
    delete_selector: DeleteSelector,
) -> Result<(), Error> {
    debug!(log: logger, "Retrieving list of users that should be deleted");

    let user_candidates = retrieve_deletion_candidate_users(inventory, delete_selector).await?;
    let user_candidate_count = user_candidates.len();

    info!(log: logger, "Identified {user_candidate_count} users for deletion");

    invite_replace_updated_by(logger, inventory, &user_candidates).await?;
    event_replace_updated_by(logger, inventory, &user_candidates).await?;

    let orphaned_rooms = delete_user_events(
        logger,
        inventory,
        authz,
        exchange_handle.clone(),
        settings,
        object_storage,
        fail_on_shared_folder_deletion_error,
        &user_candidates,
    )
    .await?;

    super::events::delete_orphaned_rooms(
        logger,
        inventory,
        authz,
        exchange_handle.clone(),
        settings,
        object_storage,
        orphaned_rooms,
        fail_on_shared_folder_deletion_error,
    )
    .await?;

    delete_users_internal(
        logger,
        inventory,
        authz,
        exchange_handle.clone(),
        settings,
        object_storage,
        &user_candidates,
    )
    .await?;

    info!(log: logger, "Deleted {} users", user_candidate_count);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn delete_users_internal(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    authz: &Authz,
    exchange_handle: ExchangeHandle,
    settings: &Settings,
    object_storage: &ObjectStorage,
    user_ids: &[UserId],
) -> Result<(), Error> {
    info!(log: logger, "Deleting users");

    let mut deleter_failures = 0usize;
    for &user_id in user_ids {
        info!(log: logger, "Deleting user {user_id}");
        let deleter = UserDeleter::new(user_id);

        if let Err(e) = deleter
            .perform(
                logger,
                inventory,
                authz,
                None,
                exchange_handle.clone(),
                settings,
                object_storage,
            )
            .await
        {
            warn!(log: logger, "Failed deletion: {}", Report::from_error(e));
            deleter_failures += 1;
        }
    }

    if deleter_failures > 0 {
        warn!(log: logger, "{deleter_failures} users could not be deleted due to errors");
    }
    Ok(())
}

/// Identify and delete events for the specified users
///
/// Returns the rooms which are orphaned as a result of the event deletion, so
/// they can be cleaned up afterwards
#[allow(clippy::too_many_arguments)]
async fn delete_user_events(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    authz: &Authz,
    exchange_handle: ExchangeHandle,
    settings: &Settings,
    object_storage: &ObjectStorage,
    fail_on_shared_folder_deletion_error: bool,
    user_candidates: &[UserId],
) -> Result<HashSet<RoomId>, Error> {
    debug!(log: logger, "Retrieving list of events that should be deleted");

    let mut event_candidates: Vec<(EventId, RoomId)> = Vec::new();

    for &user_id in user_candidates {
        let event_delete_selector = super::events::DeleteSelector::BelongingToUser(user_id);

        let mut candidates = super::events::retrieve_deletion_candidate_events(
            logger,
            inventory,
            event_delete_selector,
        )
        .await?;
        event_candidates.append(&mut candidates);
    }

    let orphaned_rooms = super::events::delete_event_candidates(
        logger,
        inventory,
        authz,
        exchange_handle,
        settings,
        object_storage,
        fail_on_shared_folder_deletion_error,
        event_candidates,
    )
    .await;

    Ok(orphaned_rooms)
}

async fn invite_replace_updated_by(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    user_candidates: &[UserId],
) -> Result<(), Error> {
    let mut touched_invites: usize = 0;
    for &user_id in user_candidates {
        let invites = inventory.get_room_invites_updated_by(user_id).await?;

        for invite in invites {
            inventory
                .update_room_invite(
                    invite.room,
                    invite.invite_code,
                    UpdateRoomInvite {
                        updated_by: Some(invite.created_by),
                        updated_at: None,
                        room: None,
                        active: None,
                        expiration: None,
                    },
                )
                .await?;
            touched_invites += 1;
        }
    }
    debug!(log: logger, "Reset the updated-by value of {} invites.", touched_invites);
    Ok(())
}

async fn event_replace_updated_by(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    user_candidates: &[UserId],
) -> Result<(), Error> {
    let mut touched_events: usize = 0;

    for &user_id in user_candidates {
        let events = inventory.get_all_events_updated_by_user(user_id).await?;

        for event in events {
            inventory
                .update_event(
                    event.id,
                    UpdateEvent {
                        title: None,
                        description: None,
                        updated_by: event.created_by,
                        updated_at: event.updated_at,
                        date: None,
                        is_adhoc: None,
                        show_meeting_details: None,
                    },
                )
                .await?;
            touched_events += 1;
        }
    }

    debug!(log: logger, "Reset the updated-by value of {} events.", touched_events);
    Ok(())
}

async fn retrieve_deletion_candidate_users(
    inventory: &mut dyn Inventory,
    delete_selector: DeleteSelector,
) -> Result<Vec<UserId>, Error> {
    let users = match delete_selector {
        DeleteSelector::DisabledBefore(delete_before) => {
            inventory.get_user_ids_disabled_before(delete_before).await
        }
    }?;

    Ok(users)
}
