// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains invite related REST endpoints.
use actix_web::{
    delete,
    web::{Data, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{error::ApiError, rooms::by_room_id::invites::RoomIdAndInviteCode};

use super::response::NoContent;
use crate::api::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Delete an invite code
///
/// The invite code will no longer be usable once it is deleted.
#[utoipa::path(
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
pub async fn delete_invite(
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
