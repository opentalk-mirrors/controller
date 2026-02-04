// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events/{event_id}/instances/{instance_id}`

use actix_web::{
    get,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{EventInstancePath, EventInstanceQuery, GetEventInstanceResponseBody},
};

use crate::utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Get an event instance
///
/// Returns the event instance resource
#[utoipa::path(
    operation_id = "get_event_instance",
    tag = "api::v1::events::instances",
    params(
        EventInstancePath,
        EventInstanceQuery,
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Event instance successfully returned",
            body = GetEventInstanceResponseBody,
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
#[get("/events/{event_id}/instances/{instance_id}")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path: Path<EventInstancePath>,
    query: Query<EventInstanceQuery>,
) -> Result<Json<GetEventInstanceResponseBody>, ApiError> {
    let response = service
        .get_event_instance(
            &current_user.into_inner(),
            path.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(Json(response))
}
