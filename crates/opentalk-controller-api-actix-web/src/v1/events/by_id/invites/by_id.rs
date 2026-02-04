// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events/{event_id}/{user_id}`

use actix_web::{
    delete,
    web::{Data, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{DeleteEventInvitePath, EventOptionsQuery},
};

use crate::{
    response::NoContent,
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
};

/// Delete an invite from an event
///
/// This will uninvite the user from the event
#[utoipa::path(
    operation_id = "delete_invite_to_event",
    tag = "api::v1::events::invites",
    params(
        DeleteEventInvitePath,
        EventOptionsQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "The user event invitation has been deleted",
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
#[delete("/events/{event_id}/invites/{user_id}")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,

    path_params: Path<DeleteEventInvitePath>,
    query: Query<EventOptionsQuery>,
) -> Result<NoContent, ApiError> {
    service
        .delete_invite_to_event(
            current_user.into_inner(),
            path_params.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(NoContent)
}
