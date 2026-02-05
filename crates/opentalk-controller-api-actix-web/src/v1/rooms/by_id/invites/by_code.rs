// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/invites/{invite_code}`

use actix_web::{
    delete, get, put,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::by_room_id::invites::{InviteResource, PutInviteRequestBody, RoomIdAndInviteCode},
};

use crate::{
    response::NoContent,
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
};

/// Get a room invite
///
/// Returns the room invite resource
#[utoipa::path(
    operation_id = "get_invite",
    tag = "api::v1::invites",
    params(RoomIdAndInviteCode),
    responses(
        (
            status = StatusCode::OK,
            description = "Successfully retrieved the room invite",
            body = InviteResource,
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
#[get("/rooms/{room_id}/invites/{invite_code}")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    path_params: Path<RoomIdAndInviteCode>,
) -> Result<Json<InviteResource>, ApiError> {
    let invite_resoruce = service
        .get_invite(path_params.room_id, path_params.invite_code)
        .await?;

    Ok(Json(invite_resoruce))
}

/// Update an invite code
///
/// Updates the field values as set in the request body.
#[utoipa::path(
    operation_id = "update_invite",
    tag = "api::v1::invites",
    params(RoomIdAndInviteCode),
    request_body = PutInviteRequestBody,
    responses(
        (
            status = StatusCode::OK,
            description = "Successfully updated the room invite",
            body = InviteResource,
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
#[put("/rooms/{room_id}/invites/{invite_code}")]
pub async fn put(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path_params: Path<RoomIdAndInviteCode>,
    update_invite: Json<PutInviteRequestBody>,
) -> Result<Json<InviteResource>, ApiError> {
    let current_user = current_user.into_inner();

    let invite_resource = service
        .update_invite(
            current_user,
            path_params.room_id,
            path_params.invite_code,
            update_invite.into_inner(),
        )
        .await?;

    Ok(Json(invite_resource))
}

/// Delete an invite code
///
/// The invite code will no longer be usable once it is deleted.
#[utoipa::path(
    operation_id = "delete_invite",
    tag = "api::v1::invites",
    params(RoomIdAndInviteCode),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "Successfully deleted the room invite",
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
#[delete("/rooms/{room_id}/invites/{invite_code}")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path_params: Path<RoomIdAndInviteCode>,
) -> Result<NoContent, ApiError> {
    let current_user = current_user.into_inner();

    service
        .delete_invite(current_user, path_params.room_id, path_params.invite_code)
        .await?;

    Ok(NoContent)
}
