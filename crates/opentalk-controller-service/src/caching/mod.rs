// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Caching functionality used by the controller service.

mod cacheable_api_error;
mod caches;

pub use cacheable_api_error::CacheableApiError;
pub use caches::Caches;
use opentalk_inventory::{Tenant, User};

/// The type that is stored in the user access token cache.
pub type UserAccessTokenResult = Result<(Tenant, User), CacheableApiError>;
