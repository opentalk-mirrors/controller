// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_signaling::ModuleData;
use url::Url;

use crate::settings_file;

#[derive(Debug, Clone, PartialEq)]
pub struct RoomServer {
    pub url: Url,

    pub api_token: String,

    pub modules: ModuleData,
}

impl From<settings_file::RoomServer> for RoomServer {
    fn from(
        settings_file::RoomServer {
            url,
            api_token,
            modules,
        }: settings_file::RoomServer,
    ) -> Self {
        Self {
            url,
            api_token,
            modules,
        }
    }
}
