// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::{
    get,
    web::{Data, ReqData},
};
use opentalk_controller_api_actix_web::v1::response::ApiResponse;
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::users::GetEventInvitesPendingResponseBody;
use serde::Deserialize;

use crate::api::{
    responses::{InternalServerError, NotFound, Unauthorized},
    v1::DefaultApiResult,
};

/// Query parameters for the `DELETE /events/{event_id}/invites/{user_id}` endpoint
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct DeleteEventInviteQuery {
    /// Flag to suppress email notification
    #[serde(default)]
    suppress_email_notification: bool,
}

/// Get information about pending invites
///
/// Returns information about pending invites for the current user
#[utoipa::path(
    responses(
        (
            status = StatusCode::OK,
            description = "Information about pending invites is returned",
            body = GetEventInvitesPendingResponseBody,
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
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
#[get("/users/me/pending_invites")]
pub async fn get_event_invites_pending(
    service: Data<dyn OpenTalkControllerService>,
    current_user: ReqData<RequestUser>,
) -> DefaultApiResult<GetEventInvitesPendingResponseBody> {
    let response = service.get_event_invites_pending(current_user.id).await?;

    Ok(ApiResponse::new(response))
}
