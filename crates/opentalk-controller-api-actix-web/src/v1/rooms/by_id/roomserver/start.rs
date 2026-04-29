// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! # DEPRECATED
//!
//! Permanently redirected to `POST /v1/rooms/{room_id}/start`
//!
//! This endpoint has been deprecated and permanently moved. Clients should update
//! their requests to use the new endpoint path.

use actix_web::{HttpResponse, http::header, post, web::Path};
use opentalk_types_common::rooms::RoomId;

#[post("/rooms/{room_id}/roomserver/start")]
#[deprecated]
pub async fn post(room_id: Path<RoomId>) -> HttpResponse {
    let room_id = room_id.into_inner();
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, format!("/v1/rooms/{}/start", room_id)))
        .finish()
}
