// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::{
    delete,
    web::{Data, Path},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::{assets::AssetId, rooms::RoomId};

use super::response::NoContent;
use crate::api::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Delete an asset from a room.
///
/// The asset is removed from the room and deleted from the storage.
#[utoipa::path(
    operation_id = "delete_room_asset",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        ("asset_id" = AssetId, description = "The id of the asset"),
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "The asset has been deleted",
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
#[delete("/rooms/{room_id}/assets/{asset_id}")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<(RoomId, AssetId)>,
) -> Result<NoContent, ApiError> {
    let (room_id, asset_id) = path.into_inner();

    service.delete_room_asset(room_id, asset_id).await?;

    Ok(NoContent)
}
