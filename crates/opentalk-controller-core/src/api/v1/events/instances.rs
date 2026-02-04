// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::{
    Either, patch,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_api_actix_web::v1::response::ApiResponse;
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{EventInstance, EventInstancePath, EventInstanceQuery, PatchEventInstanceBody},
};

use crate::api::{
    responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::NoContent,
};

/// Modifies an event instance
///
/// Patch an instance of a recurring event. This creates or modifies an exception for the event
/// at the point of time of the given instance_id.
/// Returns the patched event instance
#[utoipa::path(
    params(
        EventInstancePath,
        EventInstanceQuery,
    ),
    request_body = PatchEventInstanceBody,
    responses(
        (
            status = StatusCode::OK,
            description = "Event instance successfully updated",
            body = EventInstance,
        ),
        (
            status = StatusCode::NO_CONTENT,
            description = "The request body was empty, no changes required",
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
#[patch("/events/{event_id}/instances/{instance_id}")]
pub async fn patch_event_instance(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path: Path<EventInstancePath>,
    query: Query<EventInstanceQuery>,
    patch: Json<PatchEventInstanceBody>,
) -> Result<Either<ApiResponse<EventInstance>, NoContent>, ApiError> {
    let event_instance = service
        .patch_event_instance(
            current_user.into_inner(),
            path.into_inner(),
            query.into_inner(),
            patch.into_inner(),
        )
        .await?;

    match event_instance {
        Some(event_instance) => Ok(Either::Left(ApiResponse::new(event_instance))),
        _ => Ok(Either::Right(NoContent)),
    }
}
