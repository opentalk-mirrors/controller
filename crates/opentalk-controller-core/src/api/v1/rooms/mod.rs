// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Room related API structs and Endpoints
//!
//! The defined structs are exposed to the REST API and will be serialized/deserialized. Similar
//! structs are defined in the Database crate [`opentalk_db_storage`] for database operations.

use actix_web::{
    delete,
    web::{self, Data, Path, ReqData},
};
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{error::ApiError, rooms::by_room_id::DeleteRoomQuery};
use opentalk_types_common::rooms::RoomId;

use super::response::NoContent;
use crate::api::responses::{Forbidden, InternalServerError, Unauthorized};

/// Delete a room and its owned resources.
///
/// Deletes the room by the id if found. See the query parameters for affecting
/// the behavior of this endpoint, such as succeding even if external resources
/// cannot be successfully deleted.
#[utoipa::path(
    params(
        ("room_id" = RoomId, description = "The id of the room"),
        DeleteRoomQuery,
    ),
    responses(
        (
            status = StatusCode::NO_CONTENT,
            description = "Room was successfully deleted",
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
            status = StatusCode::INTERNAL_SERVER_ERROR,
            response = InternalServerError,
        ),
    ),
    security(
        ("BearerAuth" = []),
    ),
)]
#[delete("/rooms/{room_id}")]
pub async fn delete(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
    room_id: Path<RoomId>,
    query: web::Query<DeleteRoomQuery>,
) -> Result<NoContent, ApiError> {
    let query = query.into_inner();

    service
        .delete_room(
            current_user.into_inner(),
            room_id.into_inner(),
            query.force_delete_reference_if_external_services_fail,
        )
        .await?;

    Ok(NoContent)
}
