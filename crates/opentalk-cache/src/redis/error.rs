// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Redis error: {}", source), context(false))]
    Redis { source: ::redis::RedisError },
    #[snafu(display("Encode error: {}", source), context(false))]
    Encode { source: bincode::error::EncodeError },
    #[snafu(display("Decode error: {}", source), context(false))]
    Decode { source: bincode::error::DecodeError },
}
