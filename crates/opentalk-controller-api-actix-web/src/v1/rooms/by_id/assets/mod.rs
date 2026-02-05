// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/assets`

use actix_web::{
    get,
    web::{Data, Path, Query},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::{
    error::ApiError, pagination::PagePaginationQuery,
    rooms::by_room_id::assets::RoomsByRoomIdAssetsGetResponseBody,
};
use opentalk_types_common::rooms::RoomId;

use crate::{
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::ApiResponse,
};

pub mod by_id;

/// Get the assets associated with a room.
///
/// This returns assets that are available for a room. If no
/// pagination query is added, the default page size is used.
#[utoipa::path(
    operation_id = "room_assets",
    tag = "api::v1::assets",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        PagePaginationQuery,
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "The assets have been returned successfully",
            body = RoomsByRoomIdAssetsGetResponseBody,
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
#[get("/rooms/{room_id}/assets")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    pagination: Query<PagePaginationQuery>,
) -> Result<ApiResponse<RoomsByRoomIdAssetsGetResponseBody>, ApiError> {
    let pagination = pagination.into_inner();

    let (assets, asset_count) = service
        .get_room_assets(room_id.into_inner(), &pagination)
        .await?;

    Ok(ApiResponse::new(assets).with_page_pagination(
        pagination.per_page,
        pagination.page,
        asset_count,
    ))
}
