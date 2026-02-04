// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events`

use actix_web::{
    post,
    web::{Data, Json, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{EventOptionsQuery, EventResource, PostEventsBody},
};

use crate::utoipa::responses::{BadRequest, InternalServerError, Unauthorized};

/// Create a new event
///
/// Create a new event with the fields sent in the body.
#[utoipa::path(
    operation_id = "new_event",
    tag = "api::v1::events",
    params(EventOptionsQuery),
    responses(
        (
            status = StatusCode::CREATED,
            description = "The event has been created",
            body = EventResource,
        ),
        (
            status = StatusCode::BAD_REQUEST,
            response = BadRequest,
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
#[post("/events")]
pub async fn post(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    new_event: Json<PostEventsBody>,
    query: Query<EventOptionsQuery>,
) -> Result<Json<EventResource>, ApiError> {
    let event_resource = service
        .new_event(
            current_user.into_inner(),
            new_event.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(Json(event_resource))
}
