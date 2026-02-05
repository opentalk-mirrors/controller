// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains invite related REST endpoints.
use actix_web::{
    delete,
    web::{Data, Path, Query, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError, events::StreamingTargetOptionsQuery,
    rooms::by_room_id::streaming_targets::RoomAndStreamingTargetId,
};

use super::response::NoContent;
use crate::api::responses::{Forbidden, InternalServerError, NotFound, Unauthorized};

/// Deletes a streaming target
///
/// The streaming target is deleted from the room
#[utoipa::path(
    params(
        RoomAndStreamingTargetId,
        StreamingTargetOptionsQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "The streaming target has been deleted",
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
#[delete("/rooms/{room_id}/streaming_targets/{streaming_target_id}")]
pub async fn delete_streaming_target(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    path_params: Path<RoomAndStreamingTargetId>,
    query: Query<StreamingTargetOptionsQuery>,
) -> Result<NoContent, ApiError> {
    service
        .delete_streaming_target(
            current_user.into_inner(),
            path_params.into_inner(),
            query.into_inner(),
        )
        .await?;

    Ok(NoContent)
}
