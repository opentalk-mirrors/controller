// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! API endpoints under `v1/users/me`

use actix_web::{
    Either, patch,
    web::{Data, Json, ReqData},
};
use oauth2::AccessToken;
use opentalk_controller_service_facade::{OpenTalkControllerService, RequestUser};
use opentalk_types_api_v1::{
    error::ApiError,
    users::{PrivateUserProfile, me::PatchMeRequestBody},
};

use crate::{
    response::NoContent,
    utoipa::responses::{InternalServerError, Unauthorized},
};

/// Patch the current user's profile
///
/// Fields that are not provided in the request body will remain unchanged.
#[utoipa::path(
    request_body = PatchMeRequestBody,
    tag = "api::v1::users",
    operation_id = "patch_users_me",
    responses(
        (
            status = StatusCode::OK,
            description = "User profile was successfully updated",
            body = PrivateUserProfile
        ),
        (
            status = StatusCode::BAD_REQUEST,
            description = r"Could not modify the user's profile due to wrong
                syntax or bad values",
        ),
        (
            status = StatusCode::UNAUTHORIZED,
            response = Unauthorized,
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
#[patch("/users/me")]
pub async fn patch(
    service: Data<dyn OpenTalkControllerService>,
    access_token: ReqData<AccessToken>,
    current_user: ReqData<RequestUser>,
    patch: Json<PatchMeRequestBody>,
) -> Result<Either<Json<PrivateUserProfile>, NoContent>, ApiError> {
    let current_user = current_user.into_inner();

    let user_profile = service
        .patch_me(
            current_user.clone(),
            patch.into_inner(),
            access_token.secret(),
        )
        .await?;

    match user_profile {
        Some(user_profile) => Ok(Either::Left(Json(user_profile))),
        _ => Ok(Either::Right(NoContent)),
    }
}
