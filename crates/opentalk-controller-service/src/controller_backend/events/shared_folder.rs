// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Handles event shared folders

use std::collections::HashSet;

use chrono::{Days, NaiveDate, Utc};
use log::warn;
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::Settings;
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{EventSharedFolder, Inventory, NewEventSharedFolder};
use opentalk_nextcloud_client::{Client, ShareId, SharePermission, ShareType};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{DeleteSharedFolderQuery, PutSharedFolderQuery},
};
use opentalk_types_common::{
    events::EventId,
    shared_folders::{SharedFolder, SharedFolderAccess},
};
use snafu::Report;

use crate::{
    ControllerBackend,
    events::{notifications::notify_event_invitees_about_update, shared_folder_for_user},
};

impl ControllerBackend {
    pub(crate) async fn get_shared_folder_for_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
    ) -> Result<SharedFolder, CaptureApiError> {
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let event = inventory.get_event(event_id).await?;

        let shared_folder = SharedFolder::from(
            inventory
                .get_event_shared_folder(event_id)
                .await?
                .ok_or_else(ApiError::not_found)?,
        );

        let shared_folder = if event.created_by == current_user.id {
            shared_folder
        } else {
            shared_folder.without_write_access()
        };

        Ok(shared_folder)
    }

    pub(crate) async fn put_shared_folder_for_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
        query: PutSharedFolderQuery,
    ) -> Result<(SharedFolder, bool), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let mail_service = (!query.suppress_email_notification)
            .then(|| self.mail_service.as_ref().clone())
            .flatten();

        let (shared_folder, created) =
            put_shared_folder(&settings, event_id, inventory.as_mut()).await?;

        let (
            event,
            _invite,
            room,
            sip_config,
            _is_favorite,
            _shared_folder,
            tariff,
            _training_participation_report,
        ) = inventory
            .get_event_with_related_items(current_user.id, event_id)
            .await?;
        let tariff = self.build_tariff_resource(&tariff)?;

        if let Some(mail_service) = &mail_service {
            let shared_folder_for_user = shared_folder_for_user(
                Some(shared_folder.clone()),
                event.created_by,
                current_user.id,
            );

            let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
            let current_user = inventory.get_user(current_user.id).await?;
            let streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

            notify_event_invitees_about_update(
                &self.user_search_client,
                &settings,
                mail_service,
                current_tenant,
                current_user,
                inventory.as_mut(),
                event,
                room,
                &tariff,
                sip_config,
                shared_folder_for_user,
                streaming_targets,
            )
            .await?;
        }

        Ok((SharedFolder::from(shared_folder), created))
    }

    pub(crate) async fn delete_shared_folder_for_event(
        &self,
        current_user: RequestUser,
        event_id: EventId,
        query: DeleteSharedFolderQuery,
    ) -> Result<(), CaptureApiError> {
        let settings = self.settings_provider.get();
        let mut inventory = self.inventory_provider.get_inventory().await?;

        let mail_service = (!query.suppress_email_notification)
            .then(|| self.mail_service.as_ref().clone())
            .flatten();

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
        let tariff = self.build_tariff_resource(&tariff)?;

        if let Some(shared_folder) = shared_folder {
            let shared_folders = std::slice::from_ref(&shared_folder);
            let deletion = delete_shared_folders(&settings, shared_folders).await;

            let streaming_targets = inventory.get_room_streaming_targets(room.id).await?;

            match deletion {
                Ok(()) => {
                    inventory.delete_shared_folder_by_event_id(event_id).await?;

                    if let Some(mail_service) = &mail_service {
                        let current_tenant = inventory.get_tenant(current_user.tenant_id).await?;
                        let current_user = inventory.get_user(current_user.id).await?;

                        notify_event_invitees_about_update(
                            &self.user_search_client,
                            &settings,
                            mail_service,
                            current_tenant,
                            current_user,
                            inventory.as_mut(),
                            event,
                            room,
                            &tariff,
                            sip_config,
                            None,
                            streaming_targets,
                        )
                        .await?;
                    }

                    Ok(())
                }
                Err(e) => {
                    if query.force_delete_reference_if_shared_folder_deletion_fails {
                        warn!(
                            "Deleting local shared folder reference anyway, because \
                        `force_delete_reference_if_shared_folder_deletion_fails` is set to true"
                        );
                        inventory.delete_shared_folder_by_event_id(event_id).await?;

                        if let Some(mail_service) = &mail_service {
                            let current_tenant =
                                inventory.get_tenant(current_user.tenant_id).await?;
                            let current_user = inventory.get_user(current_user.id).await?;

                            notify_event_invitees_about_update(
                                &self.user_search_client,
                                &settings,
                                mail_service,
                                current_tenant,
                                current_user,
                                inventory.as_mut(),
                                event,
                                room,
                                &tariff,
                                sip_config,
                                None,
                                streaming_targets,
                            )
                            .await?;
                        }

                        Ok(())
                    } else {
                        Err(e.into())
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

/// Adds a shared folder to the specified event
pub async fn put_shared_folder(
    settings: &Settings,
    event_id: EventId,
    inventory: &mut dyn Inventory,
) -> Result<(EventSharedFolder, bool), CaptureApiError> {
    let shared_folder = inventory.get_event_shared_folder(event_id).await?;

    if let Some(shared_folder) = shared_folder {
        return Ok((shared_folder, false));
    }
    let shared_folder_settings = settings.shared_folder.as_ref().ok_or_else(|| {
        ApiError::bad_request().with_message("No shared folder configured for this server")
    })?;

    match shared_folder_settings {
        opentalk_controller_settings::SharedFolder::Nextcloud {
            url,
            username,
            password,
            directory,
            expiry,
        } => {
            let client =
                Client::new(url.clone(), username.clone(), password.clone()).map_err(|e| {
                    warn!("Error creating NextCloud client: {}", Report::from_error(e));
                    ApiError::internal().with_message("Error creating NextCloud client")
                })?;
            let path = format!(
                "{}/opentalk-event-{}",
                directory.trim_matches('/'),
                event_id
            );
            let user_path = format!("files/{username}/{path}");
            client.create_folder(&user_path).await.map_err(|e| {
                warn!(
                    "Error creating folder on NextCloud: {}",
                    Report::from_error(e)
                );
                ApiError::internal().with_message("Error creating folder on NextCloud")
            })?;

            let expire_date = expiry
                .as_ref()
                .map(|days| Utc::now().date_naive() + Days::new(*days));

            let write_permissions = HashSet::from([
                SharePermission::Read,
                SharePermission::Create,
                SharePermission::Update,
                SharePermission::Delete,
            ]);
            let read_permissions = HashSet::from([SharePermission::Read]);

            async fn create_share(
                client: &Client,
                path: &str,
                permissions: HashSet<SharePermission>,
                label: &str,
                password: String,
                expire_date: Option<NaiveDate>,
            ) -> Result<(ShareId, SharedFolderAccess), ApiError> {
                let mut creator = client
                    .create_share(path, ShareType::PublicLink)
                    .password(&password)
                    .label(label);
                for permission in &permissions {
                    creator = creator.permission(*permission);
                }
                if let Some(expire_date) = expire_date {
                    creator = creator.expire_date(expire_date);
                }
                let share = creator.send().await.map_err(|e| {
                    warn!(
                        "Error creating share on NextCloud: {}",
                        Report::from_error(e)
                    );
                    ApiError::internal().with_message("Error creating share on NextCloud")
                })?;

                // Workaround for NextCloud up to version 25 not processing the share permissions
                // on folder creation. We just need to change them with a subsequent update request.
                //
                // See: https://github.com/nextcloud/server/issues/32611
                if share.data.permissions != permissions {
                    _ = client
                        .update_share(share.data.id.clone())
                        .permissions(permissions)
                        .await
                        .map_err(|e| {
                            warn!(
                                "Error setting permissions for share on NextCloud: {}",
                                Report::from_error(e)
                            );
                            ApiError::internal()
                                .with_message("Error setting permissions for share on NextCloud")
                        })?;
                }

                Ok((
                    share.data.id,
                    SharedFolderAccess {
                        url: share.data.url,
                        password,
                    },
                ))
            }

            let write_password = generate_password(&client).await?;
            let read_password = generate_password(&client).await?;

            let (
                write_share_id,
                SharedFolderAccess {
                    url: write_url,
                    password: write_password,
                },
            ) = create_share(
                &client,
                &path,
                write_permissions,
                "OpenTalk read-write",
                write_password,
                expire_date,
            )
            .await?;
            let (
                read_share_id,
                SharedFolderAccess {
                    url: read_url,
                    password: read_password,
                },
            ) = create_share(
                &client,
                &path,
                read_permissions,
                "OpenTalk read-only",
                read_password,
                expire_date,
            )
            .await?;

            let shared_folder = inventory
                .try_create_event_shared_folder(NewEventSharedFolder {
                    event_id,
                    path,
                    write_share_id: write_share_id.to_string(),
                    write_url,
                    write_password,
                    read_share_id: read_share_id.to_string(),
                    read_url,
                    read_password,
                })
                .await?
                .ok_or_else(ApiError::internal)?;

            Ok((shared_folder, true))
        }
    }
}

/// Deletes the shared folders for the specified event
pub async fn delete_shared_folders(
    settings: &Settings,
    shared_folders: &[EventSharedFolder],
) -> Result<(), ApiError> {
    if shared_folders.is_empty() {
        return Ok(());
    }

    let shared_folder_settings = if let Some(settings) = settings.shared_folder.as_ref() {
        settings
    } else {
        return Err(
            ApiError::bad_request().with_message("No shared folder configured for this server")
        );
    };

    match shared_folder_settings {
        opentalk_controller_settings::SharedFolder::Nextcloud {
            url,
            username,
            password,
            ..
        } => {
            let client =
                Client::new(url.clone(), username.clone(), password.clone()).map_err(|e| {
                    warn!("Error creating NextCloud client: {}", Report::from_error(e));
                    ApiError::internal().with_message("Error creating NextCloud client")
                })?;
            for shared_folder in shared_folders {
                let path = &shared_folder.path;
                if path.trim_matches('/').is_empty() {
                    warn!(
                        "Preventing recursive deletion of empty shared folder path, this is probably harmful and not intended"
                    );
                    return Err(ApiError::internal());
                }
                let user_path = format!("files/{username}/{path}");
                if let Err(e) = client
                    .delete_share(ShareId::from(shared_folder.read_share_id.clone()))
                    .await
                {
                    warn!(
                        "Could not delete NextCloud read share: {}",
                        Report::from_error(e)
                    );
                }
                if let Err(e) = client
                    .delete_share(ShareId::from(shared_folder.write_share_id.clone()))
                    .await
                {
                    warn!(
                        "Could not delete NextCloud write share: {}",
                        Report::from_error(e)
                    );
                }
                match client.delete(&user_path).await {
                    Ok(()) | Err(opentalk_nextcloud_client::Error::FileNotFound { .. }) => {}
                    Err(e) => {
                        warn!(
                            "Error deleting folder on NextCloud: {}",
                            Report::from_error(e)
                        );
                        return Err(
                            ApiError::internal().with_message("Error deleting folder on NextCloud")
                        );
                    }
                };
            }
            Ok(())
        }
    }
}

async fn generate_password(client: &Client) -> Result<String, ApiError> {
    client.generate_password().await.map_err(|e| {
        warn!(
            "Error generating share password on NextCloud: {}",
            Report::from_error(e)
        );
        ApiError::internal().with_message("Error generating share password NextCloud")
    })
}
