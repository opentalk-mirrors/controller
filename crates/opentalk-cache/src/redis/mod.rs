// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

mod cache;
mod cache_key;
mod error;

pub type Connection = redis::aio::ConnectionManager;

pub use cache::Cache;
use cache_key::CacheKey;
pub use error::Error;
