// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/invites`

use actix_web::{
    get,
    web::{Data, Path, Query},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::{
    error::ApiError, pagination::PagePaginationQuery,
    rooms::by_room_id::invites::GetRoomsInvitesResponseBody,
};
use opentalk_types_common::rooms::RoomId;

use crate::{
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::ApiResponse,
};

/// Get all invites for a room
///
/// This returns all invites that are available for a room. If no
/// pagination query is added, the default page size is used.
#[utoipa::path(
    operation_id = "get_invites",
    tag = "api::v1::invites",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        PagePaginationQuery,
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "The invites could be loaded successfully",
            body = GetRoomsInvitesResponseBody,
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
#[get("/rooms/{room_id}/invites")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
    pagination: Query<PagePaginationQuery>,
) -> Result<ApiResponse<GetRoomsInvitesResponseBody>, ApiError> {
    let room_id = room_id.into_inner();

    let (invite_resources, invite_count) = service.get_invites(room_id, &pagination).await?;

    Ok(ApiResponse::new(invite_resources).with_page_pagination(
        pagination.per_page,
        pagination.page,
        invite_count,
    ))
}
