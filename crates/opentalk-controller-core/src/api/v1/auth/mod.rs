// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Auth related API structs and Endpoints

#![allow(deprecated)]

use actix_web::{
    get,
    web::{Data, Json},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::auth::GetLoginResponseBody;

use crate::api::responses::InternalServerError;

/// Get the configured OIDC provider
///
/// Returns the relevant information for a frontend to authenticate against the
/// configured OIDC provider for the OpenTalk service.
#[utoipa::path(
    responses(
        (
            status = StatusCode::OK,
            description = "Get information about the OIDC provider",
            body = GetLoginResponseBody,
        ),
        (
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(),
)]
#[get("/auth/login")]
pub async fn get_login(service: Data<dyn OpenTalkControllerService>) -> Json<GetLoginResponseBody> {
    Json(service.get_login().await)
}
