// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_types_signaling::ModuleData;
use serde::Deserialize;
use url::Url;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RoomServer {
    pub url: Url,

    pub api_token: String,

    pub modules: ModuleData,
}
