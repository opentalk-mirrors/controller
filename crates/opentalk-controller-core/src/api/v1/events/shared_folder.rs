// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::{
    delete,
    web::{Data, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{error::ApiError, events::DeleteSharedFolderQuery};
use opentalk_types_common::events::EventId;

use crate::api::{
    responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
    v1::response::NoContent,
};

/// Delete the shared folder of an event
///
/// Will delete the shared folder from the external system and remove the reference to it
#[utoipa::path(
    params(
        ("event_id" = EventId, description = "The id of the event"),
        DeleteSharedFolderQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "Shared folder was successfully deleted, or no shared folder had been present",
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
#[delete("/events/{event_id}/shared_folder")]
pub async fn delete_shared_folder_for_event(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    event_id: Path<EventId>,
    query: Query<DeleteSharedFolderQuery>,
) -> Result<NoContent, ApiError> {
    service
        .delete_shared_folder_for_event(
            current_user.into_inner(),
            event_id.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(NoContent)
}
