// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events/{event_id}/shared_folder`

use actix_web::{
    get,
    web::{Data, Json, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::{events::EventId, shared_folders::SharedFolder};

use crate::utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Get the shared folder for an event
///
/// Returns the shared folder for an event if available
#[utoipa::path(
    operation_id = "get_shared_folder_for_event",
    tag = "api::v1::events::shared_folder",
    params(
        ("event_id" = EventId, description = "The id of the event"),
    ),
    responses(
        (
            status = StatusCode::OK,
            description = "Shared folder returned",
            body = SharedFolder,
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
#[get("/events/{event_id}/shared_folder")]
pub async fn get(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    event_id: Path<EventId>,
) -> Result<Json<SharedFolder>, ApiError> {
    let shared_folder = service
        .get_shared_folder_for_event(current_user.into_inner(), event_id.into_inner())
        .await?;

    Ok(Json(shared_folder))
}
