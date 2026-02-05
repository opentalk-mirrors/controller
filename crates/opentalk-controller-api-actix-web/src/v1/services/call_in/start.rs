// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/services/call_in/start`

use actix_web::{
    post,
    web::{Data, Json},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    services::{PostServiceStartResponseBody, call_in::PostCallInStartRequestBody},
};

use crate::utoipa::responses::{InternalServerError, Unauthorized};

/// Starts a signaling session for call-in
///
/// Takes call-in id and pin and returns a ticket for the `/signaling` endpoint. Behaves similar to the
/// `/rooms/{room_id}/start` endpoint.
///
/// This endpoint is provided for call-in gateways to start a room connection
/// for call-in participants. The participant typically has to provide the
/// credentials (id and pin) via DTMF (the number pad).
#[utoipa::path(
    operation_id = "post_call_in_start",
    tag = "api::v1::services::call_in",
    context_path = "/services/call_in",
    request_body = PostCallInStartRequestBody,
    responses(
        (
            status = StatusCode::OK,
            description = "The dial-in participant has successfully \
                authenticated for the room. Information needed for connecting to the signaling \
                is contained in the response",
            body = PostServiceStartResponseBody,
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = "`id` and `pin` are not valid for any room.",
            body = ErrorBody,
        ),
        (
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(
        ("BearerAuth" = []),
    ),
)]
#[post("/start")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    request: Json<PostCallInStartRequestBody>,
) -> Result<Json<PostServiceStartResponseBody>, ApiError> {
    let response = service.start_call_in(request.into_inner()).await?;

    Ok(Json(response))
}
