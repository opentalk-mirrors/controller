// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/start_invited`

use actix_web::{
    post,
    web::{Data, Json, Path},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, StartRoomError};
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    rooms::by_room_id::{PostRoomsStartInvitedRequestBody, RoomsStartResponseBody},
};
use opentalk_types_common::rooms::RoomId;

use crate::utoipa::responses::{InternalServerError, NotFound};

/// Start a signaling session for an invitation code
///
/// Returns a ticket to be used with the `/signaling` endpoint. When joining a
/// room, the ticket must be provided as `Sec-WebSocket-Protocol` header field
/// when starting the WebSocket connection. When the requested room has a
/// password set, the requester must provide the correct password through the
/// requests body. When the request has no password set, the password will be
/// ignored if provided.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    tag = "api::v1::rooms",
    operation_id = "start_invited",
    request_body = PostRoomsStartInvitedRequestBody,
    responses(
        (
            status = StatusCode::OK,
            description = "Response body includes the information needed to connect to the signaling endpoint",
            body = RoomsStartResponseBody,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = r"The provided ID token is malformed or contains
                invalid claims,  no breakout rooms were found for this room, the
                breakout room id is invalid, the room doesn't exist, the guest
                does not have a valid invite for this room or legacy signaling has been
                disabled for this controller. Guests shall not be able to distinguish
                between existing rooms and rooms they don't have permission to enter,
                therefore the response is the same in these cases",
            body = ErrorBody,
            examples(
                (
                    "NoBreakoutRooms" = (
                        summary = "No breakout rooms", value = json!(ApiError::from(StartRoomError::NoBreakoutRooms).body)
                    )
                ),
                (
                    "InvalidBreakoutRoomId" = (
                        summary = "Invalid breakout room id", value = json!(ApiError::from(StartRoomError::InvalidBreakoutRoomId).body)
                    )
                ),
                (
                    "LegacySignalingDisabled" = (
                        summary = "Legacy signaling is disabled", value = json!(ApiError::from(StartRoomError::LegacySignalingDisabled).body)
                    )
                ),
                (
                    "RoomIdMismatch" = (
                        summary = "Room id mismatch", value = json!(ErrorBody::new("bad_request", "Room id mismatch"))
                    )
                ),
            ),
        ),
        (
            status = StatusCode::UNPROCESSABLE_ENTITY,
            description = "Invalid invite code",
        ),
        (
            status = StatusCode::UNPROCESSABLE_ENTITY,
            description = "Invalid body contents received",
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            body = ErrorBody,
            description = r"Either: the provided access token is expired or the
                provided id or access token is invalid. The WWW-Authenticate
                header will contain an error description 'session expired' to
                distinguish between an invalid and an expired token.
                Or: the provided password was incorrect, in which case the body
                contains more information.",
            headers(
                (
                    "www-authenticate",
                    description = "Will contain 'session expired' to distinguish between an invalid and an expired token"
                ),
            ),
            examples(
                ("WrongRoomPassword" = (
                    summary = "Wrong room password",
                    value = json!(ApiError::from(StartRoomError::WrongRoomPassword).body)
                )),
                ("ExpiredOrInvalidAccessToken" = (
                    summary = "Expired or invalid access token",
                    value = json!(
                        ApiError::unauthorized()
                        .with_message("The session for this user has expired")
                        .with_www_authenticate(opentalk_types_api_v1::error::AuthenticationError::SessionExpired)
                        .body
                    )
                )),
            ),
        ),
        (
            status = StatusCode::FORBIDDEN,
            body = ErrorBody,
            description = "The participant has been banned from entering this room",
            example = json!(ApiError::from(StartRoomError::BannedFromRoom).body),
        ),
        (
            status = StatusCode::NOT_FOUND,
            response = NotFound,
        ),
        (
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(),
)]
#[post("/rooms/{room_id}/start_invited")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    request: Json<PostRoomsStartInvitedRequestBody>,
) -> Result<Json<RoomsStartResponseBody>, ApiError> {
    let response = service
        .start_invited_room_session(room_id.into_inner(), request.into_inner())
        .await?;

    Ok(Json(response))
}
