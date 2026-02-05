// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/streaming_targets`

use actix_web::{
    get,
    web::{Data, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError, pagination::PagePaginationQuery,
    rooms::by_room_id::streaming_targets::GetRoomStreamingTargetsResponseBody,
};
use opentalk_types_common::{pagination::ItemCount, rooms::RoomId};

use crate::{
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::ApiResponse,
};

/// Lists the streaming targets of a room
///
/// Returns the streaming targets available for a room
#[utoipa::path(
    operation_id = "get_streaming_targets",
    tag = "api::v1::streaming_targets",
    params(
        PagePaginationQuery,
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "List of streaming targets successfully returned",
            body = GetRoomStreamingTargetsResponseBody,
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
#[get("/rooms/{room_id}/streaming_targets")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    room_id: Path<RoomId>,
    pagination: Query<PagePaginationQuery>,
) -> Result<ApiResponse<GetRoomStreamingTargetsResponseBody>, ApiError> {
    let response = service
        .get_streaming_targets(current_user.id, room_id.into_inner(), &pagination)
        .await?;
    let length = ItemCount::try_from(response.0.len())
        .expect("looks like we got more items than can be represented in the ItemCount type");

    Ok(ApiResponse::new(response).with_page_pagination(
        pagination.per_page,
        pagination.page,
        length,
    ))
}
