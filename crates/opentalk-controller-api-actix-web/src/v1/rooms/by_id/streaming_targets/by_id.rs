// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/streaming_targets/{streaming_target_id}`

use actix_web::{
    get,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::by_room_id::streaming_targets::{
        GetRoomStreamingTargetResponseBody, RoomAndStreamingTargetId,
    },
};

use crate::utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Gets a streaming target
///
/// Returns a single streaming target for a specific room.
#[utoipa::path(
    operation_id = "get_streaming_target",
    tag = "api::v1::streaming_targets",
    params(RoomAndStreamingTargetId),
    responses(
        (
            status = StatusCode::OK,
            description = "The streaming target has been successfully returned",
            body = GetRoomStreamingTargetResponseBody,
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
            status = StatusCode::NOT_FOUND,
            response = NotFound,
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
#[get("/rooms/{room_id}/streaming_targets/{streaming_target_id}")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path_params: Path<RoomAndStreamingTargetId>,
) -> Result<Json<GetRoomStreamingTargetResponseBody>, ApiError> {
    let response = service
        .get_streaming_target(current_user.id, path_params.into_inner())
        .await?;

    Ok(Json(response))
}
