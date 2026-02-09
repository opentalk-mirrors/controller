// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use serde::Deserialize;

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
    /// Wether the rate limit will apply
    #[serde(default)]
    pub disabled: bool,
    /// The tokens that are added to the bucket per second
    pub tokens_per_second: Option<u16>,
    /// The maximum amount of tokens that a token bucket can hold at a time
    pub token_bucket_size: Option<u16>,
}
