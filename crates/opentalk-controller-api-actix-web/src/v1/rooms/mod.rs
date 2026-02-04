// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms`

use actix_web::{
    get,
    web::{Data, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError, pagination::PagePaginationQuery, rooms::GetRoomsResponseBody,
};

use crate::{
    utoipa::responses::{InternalServerError, Unauthorized},
    v1::response::{ApiResponse, headers::PageLink},
};

pub mod by_id;

/// Get a list of rooms accessible by the requesting user
///
/// All rooms accessible to the requesting user are returned in a list. If no
/// pagination query is added, the default page size is used.
#[utoipa::path(
    params(PagePaginationQuery),
    tag = "api::v1::rooms",
    operation_id = "get_rooms",
    responses(
        (
            status = StatusCode::OK,
            description = "List of accessible rooms successfully returned",
            body = GetRoomsResponseBody,
            headers(
                ("link" = PageLink, description = "Links for paging through the results"),
            ),
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
#[get("/rooms")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    pagination: Query<PagePaginationQuery>,
) -> Result<ApiResponse<GetRoomsResponseBody>, ApiError> {
    let current_user = current_user.into_inner();
    let pagination = pagination.into_inner();

    let (rooms, room_count) = service.get_rooms(current_user.id, &pagination).await?;

    Ok(ApiResponse::new(rooms).with_page_pagination(
        pagination.per_page,
        pagination.page,
        room_count,
    ))
}
