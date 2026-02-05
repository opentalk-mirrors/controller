// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/invites/{invite_code}`

use actix_web::{
    get,
    web::{Data, Json, Path},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::{
    error::ApiError,
    rooms::by_room_id::invites::{InviteResource, RoomIdAndInviteCode},
};

use crate::utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

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
