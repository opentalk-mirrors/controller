// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use serde::Deserialize;

use crate::{
    Result,
    settings_error::{RateLimitTokenBucketSizeMissingSnafu, RateLimitTokensPerSecondMissingSnafu},
    settings_file,
};

/// Configuration for the the websocket rate limiting
///
/// The implementation uses the token bucket algorithm. Each websocket message that is sent by a participant consumes
/// one token. A websocket connection has a maximum amount of tokens that can be available at a time (the token bucket).
/// Each second, the bucket is filled with a configured amount of tokens.
///
/// The algorithm allows the configuration to have a reasonably small amount of messages per second, while still
/// allowing 'bursts' of messages until the tokens in the bucket are fully consumed.
///
/// A participant gets kicked from the conference when no tokens remain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WebSocketRateLimit {
    /// The tokens that are added to the bucket per second
    pub tokens_per_second: u16,
    /// The maximum amount of tokens that a token bucket can hold at a time
    pub token_bucket_size: u16,
}

impl WebSocketRateLimit {
    pub(crate) fn from_settings_file(
        config: Option<settings_file::WebSocketRateLimit>,
    ) -> Result<Option<Self>> {
        let Some(config) = config else {
            // enable rate limiting by default if no config was found
            return Ok(Some(Self::default()));
        };

        if config.disabled {
            return Ok(None);
        }

        let Some(tokens_per_second) = config.tokens_per_second else {
            return RateLimitTokensPerSecondMissingSnafu.fail();
        };

        let Some(token_bucket_size) = config.token_bucket_size else {
            return RateLimitTokenBucketSizeMissingSnafu.fail();
        };

        Ok(Some(Self {
            tokens_per_second,
            token_bucket_size,
        }))
    }
}

impl Default for WebSocketRateLimit {
    fn default() -> Self {
        Self {
            tokens_per_second: 10,
            token_bucket_size: 30,
        }
    }
}
