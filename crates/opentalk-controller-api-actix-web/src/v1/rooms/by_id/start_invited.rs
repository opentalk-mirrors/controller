// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/rooms/{room_id}/start_invited`

use actix_web::{HttpResponse, http::header, post, web::Path};
use opentalk_types_api_v1::rooms::by_room_id::PostRoomsRoomserverStartInvitedRequestBody;
use opentalk_types_common::rooms::RoomId;

/// Start a signaling session with the roomserver with an invitation code
///
/// This endpoint has to be called in order to get a signaling token for the roomserver. This endpoint returns the token
/// and the corresponding roomserver address. Call the  *GET `<roomserver_address>/signaling/<token>`* endpoint to
/// establish the websocket connection with the roomserver.
#[utoipa::path(
    operation_id = "start_invited",
    tag = "api::v1::rooms",
    params(
        ("room_id" = RoomId, description = "The id of the room"),
    ),
    request_body = PostRoomsRoomserverStartInvitedRequestBody,
    responses(
        (
            status = StatusCode::PERMANENT_REDIRECT,
            description = "Redirects to *POST `/v1/rooms/{room_id}/start`*",
            headers(
             ("location", description = "Target URL: `/v1/rooms/{room_id}/start`"),
            ),
        ),
    ),
    security(),
)]
#[post("/rooms/{room_id}/start_invited")]
pub async fn post(room_id: Path<RoomId>) -> HttpResponse {
    let room_id = room_id.into_inner();
    HttpResponse::PermanentRedirect()
        .insert_header((header::LOCATION, format!("/v1/rooms/{}/start", room_id)))
        .finish()
}
