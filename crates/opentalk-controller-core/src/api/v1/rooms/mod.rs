// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Room related API structs and Endpoints
//!
//! The defined structs are exposed to the REST API and will be serialized/deserialized. Similar
//! structs are defined in the Database crate [`opentalk_db_storage`] for database operations.

use actix_web::{
    delete, get, post,
    web::{self, Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser, StartRoomError};
use opentalk_types_api_v1::{
    error::{ApiError, ErrorBody},
    rooms::by_room_id::{DeleteRoomQuery, PostRoomsStartRequestBody, RoomsStartResponseBody},
};
use opentalk_types_common::{
    rooms::{RoomId, invite_codes::InviteCode},
    tariffs::TariffResource,
};

use super::response::NoContent;
use crate::api::responses::{Forbidden, InternalServerError, Unauthorized};

pub(crate) mod roomserver;

/// Delete a room and its owned resources.
///
/// Deletes the room by the id if found. See the query parameters for affecting
/// the behavior of this endpoint, such as succeding even if external resources
/// cannot be successfully deleted.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        DeleteRoomQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "Room was successfully deleted",
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
        ),
        (
            status = StatusCode::FORBIDDEN,
            response = Forbidden,
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
#[delete("/rooms/{room_id}")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    room_id: Path<RoomId>,
    query: web::Query<DeleteRoomQuery>,
) -> Result<NoContent, ApiError> {
    let query = query.into_inner();

    service
        .delete_room(
            current_user.into_inner(),
            room_id.into_inner(),
            query.force_delete_reference_if_external_services_fail,
        )
        .await?;

    Ok(NoContent)
}

/// Get a room's tariff
///
/// This returns the tariff that applies to the room, typically the tariff of
/// the room creator.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "The room's tariff was successfully retrieved",
            body = TariffResource
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
        ),
        (
            status = StatusCode::FORBIDDEN,
            response = Forbidden,
        ),
        (
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(
        ("BearerAuth" = []),
        ("InviteCode" = []),
    ),
)]
#[get("/rooms/{room_id}/tariff")]
pub async fn get_room_tariff(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    invite_code: ReqData<Option<InviteCode>>,
) -> Result<Json<TariffResource>, ApiError> {
    Ok(Json(
        service
            .get_room_tariff(&room_id, invite_code.into_inner())
            .await?,
    ))
}

/// Start a signaling session as a registered user
///
/// This endpoint has to be called in order to get a room ticket. When joining a room, the ticket
/// must be provided as a `Sec-WebSocket-Protocol` header field when starting the WebSocket
/// connection.
#[utoipa::path(
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
pub async fn start(
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
