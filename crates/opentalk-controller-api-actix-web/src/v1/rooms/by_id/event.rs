// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/event`

use actix_web::{
    get,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::{error::ApiError, rooms::by_room_id::GetRoomEventResponseBody};
use opentalk_types_common::{
    events::EventInfo,
    rooms::{RoomId, invite_codes::InviteCode},
};

use crate::utoipa::responses::{Forbidden, InternalServerError, Unauthorized};

/// Get a room's event
///
/// This returns the event with which the room is associated. Please note
/// that rooms can exist without events, in which case a `404` status will be
/// returned.
#[utoipa::path(
    operation_id = "get_room_event",
    tag = "api::v1::rooms",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "The room's event was successfully retrieved",
            body = EventInfo
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
#[get("/rooms/{room_id}/event")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    invite_code: ReqData<Option<InviteCode>>,
) -> Result<Json<GetRoomEventResponseBody>, ApiError> {
    Ok(Json(
        service
            .get_room_event(&room_id, invite_code.into_inner())
            .await?,
    ))
}
