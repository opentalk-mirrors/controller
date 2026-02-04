// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! User related API structs and Endpoints
//!
//! The defined structs are exposed to the REST API and will be serialized/deserialized. Similar
//! structs are defined in the Database crate [`opentalk_db_storage`] for database operations.

use actix_web::{
    get,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    assets::AssetSortingQuery,
    error::ApiError,
    pagination::PagePaginationQuery,
    users::{GetUserAssetsResponseBody, PrivateUserProfile, PublicUserProfile},
};
use opentalk_types_common::{tariffs::TariffResource, users::UserId};

use crate::api::{
    responses::{Forbidden, InternalServerError, Unauthorized},
    v1::ApiResponse,
};

/// Get the current user's profile
///
/// Returns the private user profile of the currently logged-in user. This
/// private profile contains information that is not visible in the public
/// profile, such as tariff status or the used storage.
#[utoipa::path(
    operation_id = "get_users_me",
    responses(
        (
            status = StatusCode::OK,
            description = "Information about the logged in user",
            body = PrivateUserProfile,
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
#[get("/users/me")]
pub async fn get_me(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
) -> Result<Json<PrivateUserProfile>, ApiError> {
    Ok(Json(service.get_me(current_user.into_inner()).await?))
}

/// Get the current user tariff information.
///
/// Returns the tariff information for the currently logged in user.
#[utoipa::path(
    responses(
        (
            status = StatusCode::OK,
            description = "Information about the tariff of the current user",
            body = TariffResource,
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
#[get("/users/me/tariff")]
pub async fn get_me_tariff(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
) -> Result<Json<TariffResource>, ApiError> {
    let resource = service.get_my_tariff(current_user.into_inner()).await?;
    Ok(Json(resource))
}

/// Get the assets associated with the user.
///
/// All assets associated to the requesting user are returned in a list. If no
/// pagination query is added, the default page size is used.
#[utoipa::path(
    params(PagePaginationQuery, AssetSortingQuery),
    responses(
        (
            status = StatusCode::OK,
            description = "List of accessible assets successfully returned",
            body = GetUserAssetsResponseBody,
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
#[get("/users/me/assets")]
pub async fn get_me_assets(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    sorting: Query<AssetSortingQuery>,
    pagination: Query<PagePaginationQuery>,
) -> Result<ApiResponse<GetUserAssetsResponseBody>, ApiError> {
    let (assets_response, asset_count) = service
        .get_my_assets(current_user.into_inner(), sorting.into_inner(), &pagination)
        .await?;

    Ok(ApiResponse::new(assets_response).with_page_pagination(
        pagination.per_page,
        pagination.page,
        asset_count,
    ))
}

/// Get a user's public profile
///
/// Returns the public profile of a user.
#[utoipa::path(
    params(
        ("user_id" = UserId, description = "The id of the user"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Information about the user",
            body = PublicUserProfile,
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
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(
        ("BearerAuth" = []),
    ),
)]
#[get("/users/{user_id}")]
pub async fn get_user(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    user_id: Path<UserId>,
) -> Result<Json<PublicUserProfile>, ApiError> {
    let user_profile = Json(
        service
            .get_user(current_user.into_inner(), user_id.into_inner())
            .await?,
    );
    Ok(user_profile)
}
