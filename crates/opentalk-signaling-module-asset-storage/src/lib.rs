// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! # Asset Storage Module
//!
//! ## Functionality
//!
//! The asset storage module provides information about the used asset storage.

use std::sync::Arc;

use opentalk_inventory::InventoryProvider;
use opentalk_signaling_core::{
    DestroyContext, Event, InitContext, ModuleContext, SignalingModule, SignalingModuleDescription,
    SignalingModuleError, SignalingModuleFeatureDescription, SignalingModuleInitData,
};
use opentalk_types_common::{assets::FileSize, modules::ModuleId, users::UserId};
use opentalk_types_signaling_asset_storage::{
    MODULE_ID, event::AssetStorageEvent, state::AssetStorageState,
};
use snafu::ResultExt as _;

pub struct AssetStorage {
    room_creator: UserId,
    inventory_provider: Arc<dyn InventoryProvider>,
}

impl SignalingModuleDescription for AssetStorage {
    const MODULE_ID: ModuleId = MODULE_ID;
    const DESCRIPTION: &'static str = "Provides infomation about the asset storage.";
    const FEATURES: &[SignalingModuleFeatureDescription] = &[];
}

#[async_trait::async_trait(? Send)]
impl SignalingModule for AssetStorage {
    const NAMESPACE: ModuleId = MODULE_ID;

    type Params = ();

    type Incoming = ();
    type Outgoing = AssetStorageEvent;
    type ExchangeMessage = ();

    type ExtEvent = ();

    type FrontendData = AssetStorageState;
    type PeerFrontendData = ();

    async fn init(
        ctx: InitContext<'_, Self>,
        _params: &Self::Params,
        _protocol: &'static str,
    ) -> Result<Option<Self>, SignalingModuleError> {
        Ok(Some(Self {
            room_creator: ctx.room.created_by,
            inventory_provider: ctx.inventory_provider().clone(),
        }))
    }

    async fn on_event(
        &mut self,
        mut ctx: ModuleContext<'_, Self>,
        event: Event<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        match event {
            Event::Joined { frontend_data, .. } => {
                if ctx.role.is_moderator() {
                    let mut inventory = self.inventory_provider.get_inventory().await?;
                    let used_storage = inventory
                        .get_user_storage_used_size_u64(self.room_creator)
                        .await?;

                    *frontend_data = Some(AssetStorageState {
                        used_storage: FileSize::try_from(used_storage)
                            .whatever_context::<&str, SignalingModuleError>("Invalid file size")?,
                    })
                }
            }
            Event::Leaving => {}
            Event::RaiseHand => {}
            Event::LowerHand => {}
            Event::ParticipantJoined(_, _) => {}
            Event::ParticipantLeft(_) => {}
            Event::ParticipantUpdated(_, _) => {}
            Event::RoleUpdated(role) => {
                if role.is_moderator() {
                    let mut inventory = self.inventory_provider.get_inventory().await?;
                    let used_storage = inventory
                        .get_user_storage_used_size_u64(self.room_creator)
                        .await?;

                    let update = AssetStorageEvent::StorageUsageUpdate {
                        used_storage: FileSize::try_from(used_storage)
                            .whatever_context::<&str, SignalingModuleError>("Invalid file size")?,
                    };
                    ctx.ws_send(update);
                }
            }
            Event::WsMessage(_) => {}
            Event::Exchange(_) => {}
            Event::Ext(_) => {}
        }

        Ok(())
    }

    async fn on_destroy(self, _ctx: DestroyContext<'_>) {}

    fn build_params(
        _init: SignalingModuleInitData,
    ) -> Result<Option<Self::Params>, SignalingModuleError> {
        Ok(Some(()))
    }
}
