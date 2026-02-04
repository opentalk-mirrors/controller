// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Response types for REST APIv1

use opentalk_types_api_v1::error::ApiError;

pub mod error;
mod ok;

pub use ok::ApiResponse;
pub use opentalk_controller_api_actix_web::response::{Created, NoContent, NotModified};

/// The default API Result
pub type DefaultApiResult<T, E = ApiError> = Result<ApiResponse<T>, E>;
