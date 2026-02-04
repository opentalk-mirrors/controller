// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/start`

use actix_web::{
    post,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser, StartRoomError};
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    rooms::by_room_id::{PostRoomsStartRequestBody, RoomsStartResponseBody},
};
use opentalk_types_common::rooms::RoomId;

use crate::utoipa::responses::InternalServerError;

/// Start a signaling session as a registered user
///
/// This endpoint has to be called in order to get a room ticket. When joining a room, the ticket
/// must be provided as a `Sec-WebSocket-Protocol` header field when starting the WebSocket
/// connection.
#[utoipa::path(
    operation_id = "start",
    tag = "api::v1::rooms",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Returns the information for joining the room",
            body = RoomsStartResponseBody,
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            description = r"The provided AccessToken is expired or the
                provided ID- or Access-Token is invalid. The WWW-Authenticate
                header will contain a error description 'session expired' to
                distinguish between an invalid and an expired token.",
            body = ErrorBody,
            headers(
                (
                    "www-authenticate",
                    description = "Will contain 'session expired' to distinguish between an invalid and an expired token"
                ),
            ),
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = "Either no breakout rooms were found for this room, the breakout room id is invalid or legacy signaling is disabled for this controller",
            body = ErrorBody,
            examples(
                ("NoBreakoutRooms" = (summary = "No breakout rooms", value = json!(ApiError::from(StartRoomError::NoBreakoutRooms).body))),
                ("InvalidBreakoutRoomId" = (summary = "Invalid breakout room id", value = json!(ApiError::from(StartRoomError::InvalidBreakoutRoomId).body))),
                ("LegacySignalingDisabled" = (summary = "Legacy signaling is disabled", value = json!(ApiError::from(StartRoomError::LegacySignalingDisabled).body)))
            ),
        ),
        (
            status = StatusCode::FORBIDDEN,
            description = "The user has not been invited to join the room or has been banned from entering this room",
            body = ErrorBody,
            examples(
                ("UserBanned" = (summary = "User has been banned from the room", value = json!(ApiError::from(StartRoomError::BannedFromRoom).body))),
                ("UserNotInvited" = (summary = "User has not been invited to join the room", value = json!(ApiError::forbidden().body))),
            ),
        ),
        (
            status = StatusCode::NOT_FOUND,
            description = "The specified room could not be found or it has no event associated with it",
            body = ErrorBody,
            example = json!(ApiError::not_found().body),
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
#[post("/rooms/{room_id}/start")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    room_id: Path<RoomId>,
    request: Json<PostRoomsStartRequestBody>,
) -> Result<Json<RoomsStartResponseBody>, ApiError> {
    let response = Json(
        service
            .start_room_session(
                current_user.into_inner(),
                room_id.into_inner(),
                request.into_inner(),
            )
            .await?,
    );

    Ok(response)
}
