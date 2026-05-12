// SPDX-License-Identifier: EUPL-1.2
// SPDX-FileCopyrightText: OpenTalk Team <mail@opentalk.eu>

use std::sync::Arc;

use actix_ws::{Message, ProtocolError};
use opentalk_controller_settings::Settings;
use opentalk_inventory::Inventory;
use opentalk_roomserver_modules::setup_registry;
use opentalk_roomserver_room::{
    ApplicationState, ModuleRegistry, RoomTaskApiError, RoomTaskContext, RoomTaskHandleError,
    RoomTaskRegistry, SignalingClientContext, TokenStore,
    settings::Task,
    storage::{
        memory_asset_storage::MemoryAssetStorage,
        memory_module_storage::MemoryModuleResourceStorage,
    },
};
use opentalk_roomserver_types::{
    api::RoomServerAccess, client_parameters::ClientParameters,
    room_parameters_patch::RoomParametersPatch, signaling::websocket::SignalingSocketMessage,
};
use opentalk_types_api_internal::module_assets::Quota;
use opentalk_types_api_v1::{error::ApiError, rooms::RoomResource};
use opentalk_types_common::{rooms::RoomId, roomserver::Token};
use tokio::sync::{Mutex, broadcast, mpsc, watch, watch::Sender};
use url::Url;

use crate::controller_backend::roomserver::{
    RoomServerBackend, SignalingHandler, build_room_parameters, websocket_adapter::WebSocketAdapter,
};

/// A roomserver backend that is running embedded in the controller.
pub(crate) struct InternalRoomServer {
    room_tasks: RoomTaskRegistry<WebSocketAdapter>,
    module_registry: Arc<ModuleRegistry>,
    app_state: Sender<ApplicationState>,
    /// A list of eligible participants and their join tokens
    token_store: Arc<Mutex<TokenStore<SignalingClientContext>>>,
    settings: Arc<Task>,
    public_url: Url,
}

impl InternalRoomServer {
    /// Create a new internal roomserver backend
    pub fn new(settings: Task, public_url: Url, shutdown: broadcast::Receiver<()>) -> Self {
        let app_state = Self::spawn_shutdown_task(shutdown);

        Self {
            room_tasks: RoomTaskRegistry::new(None),
            module_registry: Arc::new(setup_registry()),
            app_state,
            token_store: Arc::new(Mutex::new(TokenStore::new())),
            settings: Arc::new(settings),
            public_url,
        }
    }

    fn room_task_context(&self) -> RoomTaskContext {
        // TODO: replace with controller implementation of asset storage once implemented.
        let asset_storage = MemoryAssetStorage::new(Quota {
            total: None,
            used: 0,
        });
        // TODO: replace with controller implementation of module resources once implemented.
        let module_resources = MemoryModuleResourceStorage::new();

        RoomTaskContext {
            module_registry: Arc::clone(&self.module_registry),
            asset_storage: Arc::new(asset_storage),
            module_resources: Arc::new(module_resources),
            settings: Arc::clone(&self.settings),
            app_state: self.app_state.subscribe(),
        }
    }

    fn spawn_shutdown_task(mut shutdown: broadcast::Receiver<()>) -> Sender<ApplicationState> {
        let (shutdown_sender, _) = watch::channel(ApplicationState::Running);

        let sender = shutdown_sender.clone();
        _ = tokio::spawn(async move {
            _ = shutdown.recv().await;
            if let Err(err) = sender.send(ApplicationState::ShuttingDown) {
                log::debug!("Failed to send shutdown signal: {err}");
            }
        });

        shutdown_sender
    }
}

#[async_trait::async_trait]
impl RoomServerBackend for InternalRoomServer {
    #[tracing::instrument(level = "debug", skip(self, inventory, settings, room), fields(%room_id = room.id))]
    async fn request_access(
        &self,
        inventory: &mut dyn Inventory,
        settings: Arc<Settings>,
        room: RoomResource,
        client_parameters: ClientParameters,
    ) -> Result<RoomServerAccess, ApiError> {
        let room_id = room.id;
        let task_handle = self.room_tasks.get_task_handle(&room_id).await;

        if let Some(task_handle) = task_handle {
            // Room already exists
            // Refresh the idle timeout of the room
            if let Err(err) = task_handle.refresh_idle_timeout().await {
                log::error!("Failed to refresh idle timeout of room {room_id}: {err}");
                return Err(ApiError::internal().with_message("Failed to refresh idle timeout"));
            }

            // Ensure the user isn't banned
            if let Some(user_id) = client_parameters.kind.user_id() {
                task_handle.reject_if_banned(user_id).await?;
            }
        } else {
            // Room needs to be created
            let room_parameters = build_room_parameters(inventory, settings, room).await?;
            let ctx = self.room_task_context();
            self.room_tasks
                .create_if_not_exists(ctx, room_id, room_parameters.into())
                .await;
        }

        let token = self
            .token_store
            .lock()
            .await
            .create_token(SignalingClientContext::new(room_id, client_parameters));

        Ok(RoomServerAccess {
            public_url: self.public_url.clone(),
            token,
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn patch_room_parameters(
        &self,
        room_id: RoomId,
        patch: RoomParametersPatch,
    ) -> Result<(), ApiError> {
        match self.room_tasks.patch_room(room_id, patch).await {
            Ok(_) | Err(RoomTaskHandleError::ApiError(RoomTaskApiError::NotFound)) => Ok(()),
            Err(err) => {
                log::debug!("Failed to patch room {room_id}: {err}");
                Err(err.into())
            }
        }
    }
}

#[async_trait::async_trait]
impl SignalingHandler for InternalRoomServer {
    async fn consume_signaling_token(
        &self,
        token: &Token,
    ) -> Result<SignalingClientContext, ApiError> {
        self.token_store
            .lock()
            .await
            .consume_token(token)
            .ok_or_else(|| {
                log::debug!("invalid or expired token");
                ApiError::not_found()
            })
    }

    async fn accept_signaling_connection(
        &self,
        ctx: SignalingClientContext,
        incoming: mpsc::Receiver<Result<Message, ProtocolError>>,
        outgoing: mpsc::Sender<SignalingSocketMessage>,
    ) -> Result<(), ApiError> {
        let SignalingClientContext {
            room_id,
            client_parameters,
        } = ctx;

        let task_handle = self
            .room_tasks
            .get_task_handle(&room_id)
            .await
            .ok_or_else(|| {
                log::debug!("room not found");
                ApiError::not_found().with_message("Room not found")
            })?;

        let adapter = WebSocketAdapter::new(incoming, outgoing);

        task_handle
            .accept_signaling_socket(adapter, client_parameters)
            .await
            .map_err(|err| {
                log::error!("Failed to attach signaling socket to room {room_id}: {err}");
                ApiError::internal().with_message("Failed to attach signaling socket to room task")
            })?;

        Ok(())
    }
}
