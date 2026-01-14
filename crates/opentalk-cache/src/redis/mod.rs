// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

mod cache;
mod cache_key;
mod error;
mod key;
mod value;

pub type Connection = redis::aio::ConnectionManager;

pub use cache::Cache;
use cache_key::CacheKey;
pub use error::Error;
pub use key::Key;
pub use value::Value;
