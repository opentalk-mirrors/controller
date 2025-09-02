// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Functionality to delete rooms including all associated resources

use diesel_async::scoped_futures::ScopedFutureExt;
use kustos::{Authz, Resource as _, ResourceId};
use kustos_shared::access::AccessMethod;
use log::Log;
use opentalk_controller_settings::Settings;
use opentalk_inventory::{EventSharedFolder, Inventory, transaction};
use opentalk_log::{debug, warn};
use opentalk_signaling_core::{ExchangeHandle, ObjectStorage, assets::asset_key, control};
use opentalk_types_common::{
    assets::AssetId, events::EventId, module_resources::ModuleResourceId, rooms::RoomId,
    users::UserId,
};
use opentalk_types_signaling::NamespacedEvent;
use snafu::{ResultExt, ensure};

use super::{Deleter, Error};
use crate::deletion::{
    error::{ObjectDeletionSnafu, RaceConditionSnafu},
    shared_folders::delete_shared_folders,
};

/// Delete a room by id including the resources it references.
#[derive(Debug)]
pub struct RoomDeleter {
    room_id: RoomId,
    fail_on_shared_folder_deletion_error: bool,
}

impl RoomDeleter {
    /// Create a new `RoomDeleter`
    ///
    /// If `fail_on_shared_folder_deletion` is true, the deletion will fail as soon
    /// as a shared folder can not be deleted from the external storage system.
    ///
    /// Otherwise just warnings will be logged, but the deletion is considered successful.
    pub fn new(room_id: RoomId, fail_on_shared_folder_deletion_error: bool) -> Self {
        Self {
            room_id,
            fail_on_shared_folder_deletion_error,
        }
    }
}

/// A struct holding the information that was collected during the database
/// commit preparation.
#[derive(Debug)]
pub struct RoomDeleterPreparedCommit {
    resources: Vec<ResourceId>,
    linked_module_resources: Vec<ModuleResourceId>,
    linked_shared_folders: Vec<EventSharedFolder>,
}

impl RoomDeleterPreparedCommit {
    async fn detect_race_condition(
        &self,
        inventory: &mut dyn Inventory,
        room_id: RoomId,
    ) -> Result<(), Error> {
        let mut current_module_resources = inventory.get_all_module_ids_for_room(room_id).await?;
        current_module_resources.sort();

        ensure!(
            current_module_resources == self.linked_module_resources,
            RaceConditionSnafu
        );

        let mut current_shared_folders =
            inventory.get_event_shared_folders_for_room(room_id).await?;
        current_shared_folders.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        ensure!(
            current_module_resources == self.linked_module_resources,
            RaceConditionSnafu
        );

        Ok(())
    }
}

/// A struct holding the information that was collected during database commit.
#[derive(Debug)]
pub struct RoomDeleterCommitOutput {
    assets: Vec<AssetId>,
    resources: Vec<ResourceId>,
}

#[async_trait::async_trait]
impl Deleter for RoomDeleter {
    type PreparedCommit = RoomDeleterPreparedCommit;
    type CommitOutput = RoomDeleterCommitOutput;

    async fn prepare_commit(
        &self,
        _logger: &dyn Log,
        inventory: &mut dyn Inventory,
    ) -> Result<Self::PreparedCommit, Error> {
        if inventory
            .get_event_id_for_room(self.room_id)
            .await?
            .is_some()
        {
            return Err(Error::Conflict {
                message: format!(
                    "Unable to delete room with id {} due to conflicting event",
                    self.room_id
                ),
            });
        }

        let mut linked_module_resources =
            inventory.get_all_module_ids_for_room(self.room_id).await?;
        let mut linked_shared_folders = inventory
            .get_event_shared_folders_for_room(self.room_id)
            .await?;

        // Sort for improved equality comparison later on, inside the transaction.
        linked_module_resources.sort();
        linked_shared_folders.sort_by(|a, b| a.event_id.cmp(&b.event_id));

        let resources = linked_module_resources
            .iter()
            .map(|e| e.resource_id())
            .chain(
                linked_shared_folders
                    .iter()
                    .map(|f| f.event_id.resource_id().with_suffix("/shared_folder")),
            )
            .chain(associated_resource_ids(self.room_id))
            .collect::<Vec<_>>();

        Ok(RoomDeleterPreparedCommit {
            resources,
            linked_module_resources,
            linked_shared_folders,
        })
    }

    async fn check_permissions(
        &self,
        _prepared_commit: &Self::PreparedCommit,
        _logger: &dyn Log,
        authz: &Authz,
        user_id: Option<UserId>,
    ) -> Result<(), Error> {
        let Some(user_id) = user_id else {
            return Ok(());
        };

        let checked = authz
            .check_user(user_id, self.room_id.resource_id(), AccessMethod::DELETE)
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
        _inventory: &mut dyn Inventory,
        exchange_handle: ExchangeHandle,
        settings: &Settings,
    ) -> Result<(), Error> {
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
            control::exchange::global_room_all_participants(self.room_id),
            message_ttl_milliseconds,
            serde_json::to_string(&message).expect("Failed to convert namespaced to json"),
        ) {
            warn!(log: logger, "Failed to publish message to exchange, {}", e);
        }

        delete_shared_folders(
            logger,
            settings,
            &prepared_commit.linked_shared_folders,
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

        let room_id = self.room_id;

        let transaction_result: Result<(Vec<AssetId>, Vec<ResourceId>), Error> =
            transaction(inventory, |inventory| {
                async move {
                    prepared_commit
                        .detect_race_condition(inventory, room_id)
                        .await?;

                    let shared_folder_event_ids = prepared_commit
                        .linked_shared_folders
                        .iter()
                        .map(|e| e.event_id)
                        .collect::<Vec<EventId>>();

                    let mut current_assets = inventory.get_all_asset_ids_for_room(room_id).await?;
                    current_assets.sort();

                    delete_rows_associated_with_room(
                        logger,
                        inventory,
                        room_id,
                        &current_assets,
                        &shared_folder_event_ids,
                    )
                    .await?;

                    Ok((current_assets, prepared_commit.resources))
                }
                .scope_boxed()
            })
            .await;

        let (assets, resources) = transaction_result?;

        Ok(RoomDeleterCommitOutput { assets, resources })
    }

    async fn post_commit(
        &self,
        commit_output: RoomDeleterCommitOutput,
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

pub(crate) async fn delete_rows_associated_with_room(
    logger: &dyn Log,
    inventory: &mut dyn Inventory,
    room_id: RoomId,
    asset_ids: &[AssetId],
    shared_folder_event_ids: &[EventId],
) -> Result<(), opentalk_inventory::Error> {
    debug!(log: logger, "Deleting shared folders from database");
    inventory
        .delete_shared_folders_by_event_ids(shared_folder_event_ids)
        .await?;

    debug!(log: logger, "Deleting module resources from database");
    inventory
        .delete_all_module_resources_for_room(room_id)
        .await?;

    debug!(log: logger, "Deleting event from database");
    inventory.delete_event_for_room(room_id).await?;

    debug!(log: logger, "Deleting sip config from database");
    inventory.delete_room_sip_config(room_id).await?;

    debug!(log: logger, "Deleting asset information from database");
    inventory.delete_assets_by_ids(asset_ids).await?;

    debug!(log: logger, "Deleting room");
    inventory.delete_room(room_id).await?;

    Ok(())
}

fn associated_resource_ids(room_id: RoomId) -> impl IntoIterator<Item = ResourceId> {
    [
        room_id.resource_id(),
        room_id.resource_id().with_suffix("/invites"),
        room_id.resource_id().with_suffix("/invites/*"),
        room_id.resource_id().with_suffix("/streaming_targets"),
        room_id.resource_id().with_suffix("/streaming_targets/*"),
        room_id.resource_id().with_suffix("/start"),
        room_id.resource_id().with_suffix("/tariff"),
        room_id.resource_id().with_suffix("/event"),
        room_id.resource_id().with_suffix("/assets"),
        room_id.resource_id().with_suffix("/assets/*"),
    ]
}

/// Get the list of room resources for deletion of an invite code
pub fn associated_resource_ids_for_invite(
    room_id: RoomId,
) -> impl IntoIterator<Item = ResourceId> + Send {
    [
        room_id.resource_id().with_suffix("/tariff"),
        room_id.resource_id().with_suffix("/event"),
    ]
}
