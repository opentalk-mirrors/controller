// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_roomserver_types::{
    module_settings::ModuleSettings, room_parameters::AssetStorageConfig,
};
use url::Url;

use crate::settings_file;

#[derive(Debug, Clone, PartialEq)]
pub struct RoomServer {
    pub url: Url,

    pub api_token: String,

    pub asset_storage: AssetStorageConfig,

    pub modules: ModuleSettings,
}

impl From<settings_file::RoomServer> for RoomServer {
    fn from(
        settings_file::RoomServer {
            url,
            api_token,
            modules,
            asset_storage,
        }: settings_file::RoomServer,
    ) -> Self {
        Self {
            url,
            api_token,
            modules,
            asset_storage,
        }
    }
}
