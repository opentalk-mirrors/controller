// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/invites`

use actix_web::{
    get, post,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    pagination::PagePaginationQuery,
    rooms::by_room_id::invites::{
        GetRoomsInvitesResponseBody, InviteResource, PostInviteRequestBody,
    },
};
use opentalk_types_common::rooms::RoomId;

use crate::{
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::ApiResponse,
};

pub mod by_code;

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

/// Create a new invite
///
/// A new invite to the room is created with the information in the body.
#[utoipa::path(
    operation_id = "add_invite",
    tag = "api::v1::invites",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    request_body = PostInviteRequestBody,
    responses(
        (
            status = StatusCode::OK,
            description = "Successfully create a new invite",
            body = InviteResource,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = "Could not create a new invite due to wrong syntax or
                bad values, for example an invalid owner id.",
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
#[post("/rooms/{room_id}/invites")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    room_id: Path<RoomId>,
    new_invite: Json<PostInviteRequestBody>,
) -> Result<Json<InviteResource>, ApiError> {
    let current_user = current_user.into_inner();
    let room_id = room_id.into_inner();
    let new_invite = new_invite.into_inner();

    let invite_resource = service
        .create_invite(current_user, room_id, new_invite)
        .await?;

    Ok(Json(invite_resource))
}
