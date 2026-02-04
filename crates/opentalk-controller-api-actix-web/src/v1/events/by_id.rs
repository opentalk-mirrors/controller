// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events/{event_id}`

use actix_web::{
    get,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{EventResource, GetEventQuery},
};
use opentalk_types_common::events::EventId;

use crate::utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Get an event
///
/// Returns the event resource for the given id
#[utoipa::path(
    operation_id = "get_event",
    tag = "api::v1::events",
    params(
        GetEventQuery,
        ("event_id" = EventId, description = "The id of the event"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Event was successfully retrieved",
            body = EventResource
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
#[get("/events/{event_id}")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    event_id: Path<EventId>,
    query: Query<GetEventQuery>,
) -> Result<Json<EventResource>, ApiError> {
    let event_resource = service
        .get_event(
            current_user.into_inner(),
            event_id.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(Json(event_resource))
}
