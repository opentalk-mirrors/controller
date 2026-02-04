// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::{
    delete,
    web::{Data, Path},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::rooms::RoomId;

use crate::api::{
    responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::NoContent,
};

/// Delete the SIP configuration of a room.
///
/// This removes the dial-in functionality from the room.
#[utoipa::path(
    operation_id = "delete_room_sip",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "The SIP configuration was successfully deleted",
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
#[delete("/rooms/{room_id}/sip")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    room_id: Path<RoomId>,
) -> Result<NoContent, ApiError> {
    service.delete_sip_config(room_id.into_inner()).await?;

    Ok(NoContent)
}
