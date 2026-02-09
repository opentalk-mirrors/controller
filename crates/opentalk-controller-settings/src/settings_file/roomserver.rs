// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_roomserver_types::module_settings::ModuleSettings;
use opentalk_service_auth::ApiKey;
use serde::Deserialize;
use url::Url;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RoomServer {
    pub url: Url,

    pub api_key: ApiKey,

    pub modules: ModuleSettings,
}
