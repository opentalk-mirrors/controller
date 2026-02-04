// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/events/{event_id}/invites/email`

use actix_web::{
    delete,
    web::{Data, Json, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    events::{DeleteEmailInviteBody, EventOptionsQuery},
};
use opentalk_types_common::events::EventId;

use crate::{
    response::NoContent,
    utoipa::responses::{Forbidden, InternalServerError, NotFound, Unauthorized},
};

/// Delete an invite from an event
///
/// Delete/Withdraw an event invitation using the email address as the identifier.
///
/// This will also withdraw invites from registered users if the provided email address matches theirs.
#[utoipa::path(
    operation_id = "delete_email_invite_to_event",
    tag = "api::v1::events::invites",
    request_body = DeleteEmailInviteBody,
    params(
        ("event_id" = EventId, description = "The id of the event"),
        EventOptionsQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "The email event invitation has been deleted",
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
#[delete("/events/{event_id}/invites/email")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path: Path<EventId>,
    query: Query<EventOptionsQuery>,
    body: Json<DeleteEmailInviteBody>,
) -> Result<NoContent, ApiError> {
    service
        .delete_email_invite_to_event(
            current_user.into_inner(),
            path.into_inner(),
            body.into_inner().email,
            query.into_inner(),
        )
        .await?;

    Ok(NoContent)
}
