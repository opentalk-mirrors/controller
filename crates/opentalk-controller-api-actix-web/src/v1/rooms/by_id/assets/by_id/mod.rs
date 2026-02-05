// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/assets/{asset_id}`

use actix_web::{
    HttpResponse, delete, get,
    http::StatusCode,
    web::{Data, Path},
};
use opentalk_controller_service_facade::OpenTalkControllerService;
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::{assets::AssetId, rooms::RoomId};

use crate::{
    response::NoContent,
    utoipa::responses::{BinaryData, Forbidden, InternalServerError, NotFound, Unauthorized},
};

pub mod download;
pub mod proxy;

/// Get a specific asset inside a room.
///
/// This will return the plain asset contents, e.g. the binary file contents or
/// whatever else is stored inside the asset storage.
#[utoipa::path(
    operation_id = "room_asset",
    tag = "api::v1::assets",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        ("asset_id" = AssetId, description = "The id of the asset"),
    ),
    responses(
        (
            status = StatusCode::OK,
            response = BinaryData,
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
#[get("/rooms/{room_id}/assets/{asset_id}")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<(RoomId, AssetId)>,
) -> Result<HttpResponse, ApiError> {
    let (room_id, asset_id) = path.into_inner();

    let stream = service.get_room_asset(room_id, asset_id).await?;

    Ok(HttpResponse::build(StatusCode::OK).streaming(stream))
}

/// Delete an asset from a room.
///
/// The asset is removed from the room and deleted from the storage.
#[utoipa::path(
    operation_id = "delete_room_asset",
    tag = "api::v1::assets",
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
