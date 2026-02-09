// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_roomserver_types::{
    module_settings::ModuleSettings,
    rate_limit::{self, RateLimitSettings},
};
use opentalk_service_auth::ApiKey;
use url::Url;

use crate::settings_file::{self, WebSocketRateLimit};

#[derive(Debug, Clone, PartialEq)]
pub struct RoomServer {
    pub url: Url,

    pub api_key: ApiKey,

    pub modules: ModuleSettings,

    pub websocket_rate_limit: Option<RateLimitSettings>,
}

impl From<settings_file::RoomServer> for RoomServer {
    fn from(
        settings_file::RoomServer {
            url,
            api_key,
            modules,
            websocket_rate_limit,
        }: settings_file::RoomServer,
    ) -> Self {
        Self {
            url,
            api_key,
            modules,
            websocket_rate_limit: rate_limit_from_settings_file(websocket_rate_limit),
        }
    }
}

fn rate_limit_from_settings_file(config: Option<WebSocketRateLimit>) -> Option<RateLimitSettings> {
    let Some(config) = config else {
        return Some(RateLimitSettings::default());
    };

    if config.disabled {
        return None;
    }

    Some(RateLimitSettings {
        tokens_per_second: config
            .tokens_per_second
            .unwrap_or(rate_limit::DEFAULT_TOKENS_PER_SECOND),
        token_bucket_size: config
            .token_bucket_size
            .unwrap_or(rate_limit::DEFAULT_TOKEN_BUCKET_SIZE),
    })
}
