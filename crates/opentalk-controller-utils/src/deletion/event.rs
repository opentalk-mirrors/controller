// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Functionality to delete events including all associated resources

use diesel_async::scoped_futures::ScopedFutureExt;
use kustos::{Authz, Resource as _, ResourceId};
use kustos_shared::access::AccessMethod;
use log::Log;
use opentalk_controller_settings::Settings;
use opentalk_inventory::{EventSharedFolder, Inventory, transaction};
use opentalk_log::{debug, warn};
use opentalk_signaling_core::{ExchangeHandle, ObjectStorage, assets::asset_key, control};
use opentalk_types_common::{assets::AssetId, events::EventId, users::UserId};
use opentalk_types_signaling::NamespacedEvent;
use snafu::{ResultExt, ensure};

use super::{Deleter, Error, shared_folders::delete_shared_folders};
use crate::deletion::{
    error::{ObjectDeletionSnafu, RaceConditionSnafu},
    room::delete_rows_associated_with_room,
};

/// Delete an event by id including the corresponding room and resources it
/// references.
#[derive(Debug)]
pub struct EventDeleter {
    event_id: EventId,
    fail_on_shared_folder_deletion_error: bool,
}

impl EventDeleter {
    /// Create a new `EventDeleter`
    ///
    /// If `fail_on_shared_folder_deletion` is true, the deletion will fail as soon
    /// as a shared folder can not be deleted from the external storage system.
    ///
    /// Otherwise just warnings will be logged, but the deletion is considered successful.
    pub fn new(event_id: EventId, fail_on_shared_folder_deletion_error: bool) -> Self {
        Self {
            event_id,
            fail_on_shared_folder_deletion_error,
        }
    }
}

/// A struct holding the information that was collected during the database
/// commit preparation.
#[derive(Debug)]
pub struct EventDeleterPreparedCommit {
    resources: Vec<ResourceId>,
    linked_shared_folder: Option<EventSharedFolder>,
}

impl EventDeleterPreparedCommit {
    async fn detect_race_condition(
        &self,
        inventory: &mut dyn Inventory,
        event_id: EventId,
    ) -> Result<(), Error> {
        let current_shared_folder = inventory.get_event_shared_folder(event_id).await?;
        ensure!(
            current_shared_folder == self.linked_shared_folder,
            RaceConditionSnafu
        );

        Ok(())
    }
}

/// A struct holding the information that was collected during database commit.
#[derive(Debug)]
pub struct EventDeleterCommitOutput {
    assets: Vec<AssetId>,
    resources: Vec<ResourceId>,
}

#[async_trait::async_trait]
impl Deleter for EventDeleter {
    type PreparedCommit = EventDeleterPreparedCommit;
    type CommitOutput = EventDeleterCommitOutput;

    async fn prepare_commit(
        &self,
        _logger: &dyn Log,
        inventory: &mut dyn Inventory,
    ) -> Result<Self::PreparedCommit, Error> {
        let linked_shared_folder = inventory.get_event_shared_folder(self.event_id).await?;

        let resources = associated_resource_ids(self.event_id)
            .into_iter()
            .chain(
                linked_shared_folder
                    .iter()
                    .map(|f| f.event_id.resource_id().with_suffix("/shared_folder")),
            )
            .collect::<Vec<_>>();

        Ok(EventDeleterPreparedCommit {
            resources,
            linked_shared_folder,
        })
    }

    async fn check_permissions(
        &self,
        _prepared_commit: &Self::PreparedCommit,
        _logger: &dyn Log,
        authz: &Authz,
        user_id: Option<UserId>,
    ) -> Result<(), Error> {
        let user_id = match user_id {
            Some(user_id) => user_id,
            None => return Ok(()),
        };

        let event_id = self.event_id;
        let checked = authz
            .check_user(user_id, event_id.resource_id(), AccessMethod::DELETE)
            .await?;

        if !checked {
            return Err(Error::Forbidden);
        }

        Ok(())
    }

    async fn pre_commit(
        &self,
        prepared_commit: &Self::PreparedCommit,
        logger: &dyn Log,
        inventory: &mut dyn Inventory,
        exchange_handle: ExchangeHandle,
        settings: &Settings,
    ) -> Result<(), Error> {
        let event = inventory.get_event(self.event_id).await?;
        let room_id = event.room;

        let message = NamespacedEvent {
            module: control::MODULE_ID,
            timestamp: opentalk_types_common::time::Timestamp::now(),
            payload: control::exchange::Message::RoomDeleted,
        };

        let message_ttl_milliseconds = settings
            .rabbit_mq
            .as_ref()
            .map(|v| v.message_ttl_milliseconds())
            .unwrap_or_default();

        if let Err(e) = exchange_handle.publish(
            control::exchange::global_room_all_participants(room_id),
            message_ttl_milliseconds,
            serde_json::to_string(&message).expect("Failed to convert namespaced to json"),
        ) {
            warn!(log: logger, "Failed to publish message to exchange, {}", e);
        }

        delete_shared_folders(
            logger,
            settings,
            prepared_commit.linked_shared_folder.as_slice(),
            self.fail_on_shared_folder_deletion_error,
        )
        .await?;
        Ok(())
    }

    async fn commit_to_inventory(
        &self,
        prepared_commit: Self::PreparedCommit,
        logger: &dyn Log,
        inventory: &mut dyn Inventory,
    ) -> Result<Self::CommitOutput, Error> {
        debug!(log: logger, "Deleting all database resources");

        let event_id = self.event_id;
        let event = inventory.get_event(event_id).await?;
        let room_id = event.room;

        let transaction_result: Result<(Vec<AssetId>, Vec<ResourceId>), Error> =
            transaction(inventory, |inventory| {
                async move {
                    prepared_commit
                        .detect_race_condition(inventory, event_id)
                        .await?;

                    let mut current_assets = inventory.get_all_asset_ids_for_room(room_id).await?;
                    current_assets.sort();

                    delete_rows_associated_with_room(
                        &logger,
                        inventory,
                        room_id,
                        &current_assets,
                        &[event_id],
                    )
                    .await?;

                    Ok((current_assets, prepared_commit.resources))
                }
                .scope_boxed()
            })
            .await;

        let (assets, resources) = transaction_result?;

        Ok(EventDeleterCommitOutput { assets, resources })
    }

    async fn post_commit(
        &self,
        commit_output: EventDeleterCommitOutput,
        logger: &dyn Log,
        _settings: &Settings,
        authz: &Authz,
        storage: &ObjectStorage,
    ) -> Result<(), Error> {
        debug!(
            log: logger,
            "Deleting {} asset(s) from the storage",
            commit_output.assets.len()
        );
        for asset_id in commit_output.assets {
            debug!(log: logger, "Deleting asset {asset_id} from the storage");
            storage
                .delete(asset_key(&asset_id))
                .await
                .context(ObjectDeletionSnafu)?;
        }

        debug!(log: logger, "Deleting auth information");
        let _removed_count = authz
            .remove_explicit_resources(commit_output.resources)
            .await?;

        Ok(())
    }
}

pub fn associated_resource_ids(event_id: EventId) -> impl IntoIterator<Item = ResourceId> {
    [
        event_id.resource_id(),
        event_id.resource_id().with_suffix("/instances"),
        event_id.resource_id().with_suffix("/instances/*"),
        event_id.resource_id().with_suffix("/invites"),
        event_id.resource_id().with_suffix("/invites/*"),
        event_id.resource_id().with_suffix("/invite"),
        event_id.resource_id().with_suffix("/reschedule"),
        event_id.resource_id().with_suffix("/shared_folder"),
        ResourceId::from(format!("/users/me/event_favorites/{event_id}")),
    ]
}
