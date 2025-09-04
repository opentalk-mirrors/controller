// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_signaling_core::{
    DestroyContext, Event, InitContext, ModuleContext, SignalingModule, SignalingModuleDescription,
    SignalingModuleError, SignalingModuleFeatureDescription, SignalingModuleInitData,
};
use opentalk_types_common::{
    features::{CALL_IN_FEATURE_ID, GUESTS_ALLOWED_FEATURE_ID, STORAGE_UPGRADABLE_FEATURE_ID},
    modules::{CORE_MODULE_ID, ModuleId},
};

pub struct Core {}

impl SignalingModuleDescription for Core {
    const MODULE_ID: ModuleId = CORE_MODULE_ID;
    const DESCRIPTION: &'static str = "Handles core meeting functionality";

    const FEATURES: &[SignalingModuleFeatureDescription] = &[
        SignalingModuleFeatureDescription {
            feature_id: CALL_IN_FEATURE_ID,
            description: "Enables telephone call-in functionality for meetings.",
        },
        SignalingModuleFeatureDescription {
            feature_id: GUESTS_ALLOWED_FEATURE_ID,
            description: "Allow guest access. With this feature enabled, guests are allowed to join meetings through invite links. When turned off, no invite links are generated, and invite links that existed before will be invalid.",
        },
        SignalingModuleFeatureDescription {
            feature_id: STORAGE_UPGRADABLE_FEATURE_ID,
            description: "Communicates to the frontend that the user's storage can be upgraded. Frontend will then show the corresponding link to the account management if the user's storage is close to the limit. This feature is usually configured differently the tariffs. If a user has a tariff which already provides the maximum available storage space, then that feature should be disabled. For all other tariffs it should be on.",
        },
    ];
}

#[async_trait::async_trait(?Send)]
impl SignalingModule for Core {
    const NAMESPACE: ModuleId = CORE_MODULE_ID;

    type Params = ();
    type Incoming = ();
    type Outgoing = ();
    type ExchangeMessage = ();
    type ExtEvent = ();
    type FrontendData = ();
    type PeerFrontendData = ();

    async fn init(
        _ctx: InitContext<'_, Self>,
        _params: &Self::Params,
        _protocol: &'static str,
    ) -> Result<Option<Self>, SignalingModuleError> {
        Ok(Some(Self {}))
    }

    async fn on_event(
        &mut self,
        mut _ctx: ModuleContext<'_, Self>,
        _event: Event<'_, Self>,
    ) -> Result<(), SignalingModuleError> {
        Ok(())
    }

    async fn on_destroy(self, _ctx: DestroyContext<'_>) {}

    fn build_params(
        _init: SignalingModuleInitData,
    ) -> Result<Option<Self::Params>, SignalingModuleError> {
        Ok(Some(()))
    }
}
