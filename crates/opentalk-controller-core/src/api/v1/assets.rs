// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_http::StatusCode;
use actix_web::{
    HttpRequest, HttpResponse, delete, get, post,
    web::{Data, Path, Payload, Query},
};
use futures::TryStreamExt;
use opentalk_controller_service_facade::{AssetDownloadProxyStream, OpenTalkControllerService};
use opentalk_signaling_core::{ObjectStorageError, assets::NewAssetFileName};
use opentalk_types_api_v1::{
    error::ApiError,
    pagination::PagePaginationQuery,
    rooms::by_room_id::assets::{
        AssetDownloadQuery, AssetDownloadResponseBody, PostAssetQuery, PostAssetResponseBody,
        RoomsByRoomIdAssetsGetResponseBody,
    },
};
use opentalk_types_common::{assets::AssetId, rooms::RoomId, time::Timestamp};
use serde::{Deserialize, Serialize};

use super::{ApiResponse, DefaultApiResult, response::NoContent};
use crate::api::responses::{BinaryData, Forbidden, InternalServerError, NotFound, Unauthorized};

/// Get the assets associated with a room.
///
/// This returns assets that are available for a room. If no
/// pagination query is added, the default page size is used.
#[utoipa::path(
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
pub async fn room_assets(
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

/// Get a specific asset inside a room.
///
/// This will return the plain asset contents, e.g. the binary file contents or
/// whatever else is stored inside the asset storage.
#[utoipa::path(
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
pub async fn room_asset(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<(RoomId, AssetId)>,
) -> Result<HttpResponse, ApiError> {
    let (room_id, asset_id) = path.into_inner();

    let stream = service.get_room_asset(room_id, asset_id).await?;

    Ok(HttpResponse::build(StatusCode::OK).streaming(stream))
}

/// Get the controller download path for an asset.
///
/// Returns a path for downloading the asset via the controller.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        ("asset_id" = AssetId, description = "The id of the asset"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Returns JSON with the controller download path",
            body = AssetDownloadResponseBody,
        ),
        (
            status = StatusCode::FOUND,
            description = "Redirects to the controller download path",
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
#[get("/rooms/{room_id}/assets/{asset_id}/download")]
pub async fn room_asset_download(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<(RoomId, AssetId)>,
    query: Query<AssetDownloadQuery>,
) -> Result<HttpResponse, ApiError> {
    let (room_id, asset_id) = path.into_inner();
    let query = query.into_inner();

    let token = service
        .get_room_asset_proxy_download_token(room_id, asset_id)
        .await?;
    let url = format!("proxy?token={token}");

    let mut response = if query.redirect.unwrap_or(true) {
        let mut res = HttpResponse::build(StatusCode::FOUND);

        res.insert_header((actix_web::http::header::LOCATION, url.clone()));

        res
    } else {
        HttpResponse::build(StatusCode::OK)
    };

    Ok(response.json(AssetDownloadResponseBody { url }))
}

/// Query parameters for *GET /rooms/{room_id}/assets/{asset_id}/proxy*
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AssetProxyDownloadQuery {
    /// The token required for the asset download
    token: String,
}

/// Stream an asset via the controller proxy.
///
/// This endpoint forwards the request to object storage using the object storage
/// query parameters supplied by the client.
#[utoipa::path(
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
    security(("BearerAuth" = [])),
)]
#[get("/rooms/{room_id}/assets/{asset_id}/proxy")]
pub async fn proxy_download(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<(RoomId, AssetId)>,
    req: HttpRequest,
    query: Query<AssetProxyDownloadQuery>,
) -> Result<HttpResponse, ApiError> {
    let (_room_id, asset_id) = path.into_inner();
    let query = query.into_inner();

    let range_header = req
        .headers()
        .get(actix_web::http::header::RANGE)
        .map(|v| v.to_str())
        .transpose()
        .unwrap()
        .map(ToString::to_string);

    let AssetDownloadProxyStream {
        status,
        headers,
        stream,
    } = service
        .get_asset_proxy_download_stream(asset_id, query.token, range_header)
        .await?;

    let mut response = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status).unwrap_or(actix_web::http::StatusCode::OK),
    );

    for (k, v) in headers {
        response.append_header((k, v));
    }

    Ok(response.streaming(stream.map_err(actix_web::error::ErrorInternalServerError)))
}

/// Create an asset for a room from an uploaded file
///
/// The asset is attached to the room and saved in the storage.
#[utoipa::path(
    operation_id = "create_room_asset",
    request_body(
        content = String,
        content_type = "application/octet-stream",
        description = "The contents of the file",
    ),
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        PostAssetQuery,
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "The asset has been created successfully",
            body = PostAssetResponseBody,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = "Storage quota has been exceeded",
        ),
        (
            status = StatusCode::NOT_FOUND,
            description = "The associated room was not found",
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
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
#[post("/rooms/{room_id}/assets")]
pub async fn create(
    service: Data<dyn OpenTalkControllerService>,
    path: Path<RoomId>,
    query: Query<PostAssetQuery>,
    data: Payload,
) -> DefaultApiResult<PostAssetResponseBody, ApiError> {
    let room_id = path.into_inner();
    let query = query.into_inner();

    let filename = NewAssetFileName::new_with_event_title(
        query.event_title,
        query.kind,
        Timestamp::now(),
        query.file_extension,
    );

    let data = data.map_err(|e| ObjectStorageError::Other {
        message: "Upload error".to_string(),
        source: Some(e.into()),
    });

    let (resource, _) = service
        .create_room_asset(room_id, filename, query.namespace, Box::new(data))
        .await?;

    let response = PostAssetResponseBody(resource);

    Ok(ApiResponse::new(response))
}

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
