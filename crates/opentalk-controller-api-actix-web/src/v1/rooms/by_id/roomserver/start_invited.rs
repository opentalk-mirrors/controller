// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/roomserver/start_invited`

use actix_web::{
    post,
    web::{Data, Json, Path},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, StartRoomError};
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    rooms::by_room_id::{PostRoomsRoomserverStartInvitedRequestBody, RoomserverStartResponseBody},
};
use opentalk_types_common::rooms::RoomId;

use crate::utoipa::responses::InternalServerError;

/// Start a signaling session with the roomserver with an invitation code
///
/// This endpoint has to be called in order to get a signaling token for the roomserver. This endpoint returns the token
/// and the corresponding roomserver address. Call the  *GET `<roomserver_address>/signaling/<token>`* endpoint to
/// establish the websocket connection with the roomserver.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    request_body = PostRoomsRoomserverStartInvitedRequestBody,
    responses(
        (
            status = StatusCode::OK,
            description = "Returns the roomserver token and roomserver address",
            body = RoomserverStartResponseBody,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = r"The provided ID token is malformed or contains
                invalid claims,  no breakout rooms were found for this room, the
                breakout room id is invalid, the room doesn't exist, the guest
                does not have a valid invite for this room or when calling this
                endpoint on a controller where no roomserver is configured. Guests
                shall not be able to distinguish between existing rooms and rooms
                they don't have permission to enter, therefore the response is the
                same in these cases.",
            body = ErrorBody,
            examples(
                (
                    "RoomIdMismatch" = (
                        summary = "Room id mismatch", value = json!(ErrorBody::new("bad_request", "Room id mismatch"))
                    )
                ),
                (
                    "RoomserverSignalingDisabled" = (
                        summary = "Roomserver signaling is disabled", value = json!(ApiError::from(StartRoomError::RoomserverSignalingDisabled).body)
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
            description = "The specified room could not be found or it has no event associated with it",
            body = ErrorBody,
            example = json!(ApiError::not_found().body),
        ),
        (
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(),
)]
#[post("/rooms/{room_id}/roomserver/start_invited")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    request: Json<PostRoomsRoomserverStartInvitedRequestBody>,
) -> Result<Json<RoomserverStartResponseBody>, ApiError> {
    let response = Json(
        service
            .start_invited_roomserver_room_session(room_id.into_inner(), request.into_inner())
            .await?,
    );

    Ok(response)
}
