// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! # DEPRECATED
//!
//! API endpoints under `v1/rooms/{room_id}/roomserver/start_invited`
//!
//! This endpoint has been permanently moved to `/v1/rooms/{room_id}/start_invited`.

use actix_web::{HttpResponse, http::header, post, web::Path};
use opentalk_types_common::rooms::RoomId;

#[post("/rooms/{room_id}/roomserver/start_invited")]
#[deprecated]
pub async fn post(room_id: Path<RoomId>) -> HttpResponse {
    let room_id = room_id.into_inner();
    HttpResponse::MovedPermanently()
        .insert_header((
            header::LOCATION,
            format!("/v1/rooms/{}/start_invited", room_id),
        ))
        .finish()
}
