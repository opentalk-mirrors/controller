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

/// RoomServer settings
#[derive(Debug, Clone, PartialEq)]
pub struct RoomServer {
    /// The service URL the RoomServer
    pub url: Url,

    /// The API key to access the RoomServer
    pub api_key: ApiKey,

    /// Settings regarding the RoomServer modules
    pub modules: ModuleSettings,

    /// Rate limit settings for RoomServer WebSocket connections. If `None`, the WebSocket rate limit is disabled.
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
    let WebSocketRateLimit {
        disabled,
        tokens_per_second,
        token_bucket_size,
        // Do not use rest pattern (`..`) here. We ensure that all fields are used by destructuring `WebSocketRateLimit`
    } = config;

    if disabled {
        return None;
    }

    Some(RateLimitSettings {
        tokens_per_second: tokens_per_second.unwrap_or(rate_limit::DEFAULT_TOKENS_PER_SECOND),
        token_bucket_size: token_bucket_size.unwrap_or(rate_limit::DEFAULT_TOKEN_BUCKET_SIZE),
    })
}
