// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/roomserver/start`

use actix_web::{
    post,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser, StartRoomError};
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    rooms::by_room_id::{PostRoomsRoomserverStartRequestBody, RoomserverStartResponseBody},
};
use opentalk_types_common::rooms::RoomId;

use crate::utoipa::responses::InternalServerError;

/// Start a signaling session with the roomserver as a registered user
///
/// This endpoint has to be called in order to get a signaling token for the roomserver. This endpoint returns the token
/// and the corresponding roomserver address. Call the  *GET `<roomserver_address>/signaling/<token>`* endpoint to
/// establish the websocket connection with the roomserver.
#[utoipa::path(
    operation_id = "roomserver_start",
    tag = "api::v1::rooms",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Returns the roomserver token and roomserver address",
            body = RoomserverStartResponseBody,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = r"Returned when calling this endpoint on a controller where no roomserver is configured",
            body = ErrorBody,
            examples(
                (
                    "RoomserverSignalingDisabled" = (
                        summary = "Roomserver signaling is disabled", value = json!(ApiError::from(StartRoomError::RoomserverSignalingDisabled).body)
                    )
                ),
            ),
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
            status = StatusCode::FORBIDDEN,
            description = "The user has not been invited to join the room or has been banned from entering this room",
            body = ErrorBody,
            examples(
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
#[post("/rooms/{room_id}/roomserver/start")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,

    room_id: Path<RoomId>,
    request: Json<PostRoomsRoomserverStartRequestBody>,
) -> Result<Json<RoomserverStartResponseBody>, ApiError> {
    let response = Json(
        service
            .start_roomserver_room_session(
                current_user.into_inner(),
                room_id.into_inner(),
                request.into_inner(),
            )
            .await?,
    );

    Ok(response)
}
