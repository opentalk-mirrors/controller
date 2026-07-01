// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Handles user Authentication in API requests
use core::future::ready;
use std::{
    future::{Future, Ready},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_http::Payload;
use actix_web::{
    HttpMessage, ResponseError,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::{Error, PayloadError},
    http::{Method, header::Header},
    web::Data,
};
use actix_web_httpauth::headers::authorization::Authorization;
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use openidconnect::AccessToken;
use opentalk_controller_api_authorization::authorization::Authorizer;
use opentalk_controller_service::oidc::{Cache, OidcTokenHandler};
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::SettingsProvider;
use opentalk_inventory::{InventoryProvider, User};
use opentalk_types_api_v1::error::{ApiError, AuthenticationError};
use opentalk_types_common::rooms::invite_codes::InviteCode;
use snafu::Report;
use tracing_futures::Instrument;

use crate::api::v1::middleware::user_auth::bearer_or_invite_code::BearerOrInviteCode;

mod access_token;
mod bearer_or_invite_code;
mod provisioning;

/// Route pattern for the room-start endpoint. Kept in sync with the
/// `#[post("/rooms/{room_id}/start")]` handler; the pattern includes the `/v1` scope prefix.
const ROOM_START_PATTERN: &str = "/v1/rooms/{room_id}/start";

/// Maximum size (in bytes) of the body we are willing to buffer when peeking.
const PEEK_BODY_LIMIT: usize = 2 * 1024 * 1024;

/// Middleware factory
///
/// Transforms into [`OidcAuthMiddleware`]
pub struct OidcAuth {
    pub settings_provider: SettingsProvider,
    pub inventory_provider: Data<dyn InventoryProvider>,
    pub authorizer: Data<Authorizer>,
    pub oidc_ctx: Data<dyn OidcTokenHandler>,
}

impl<S> Transform<S, ServiceRequest> for OidcAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = OidcAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(OidcAuthMiddleware {
            service: Rc::new(service),
            settings_provider: self.settings_provider.clone(),
            authorizer: self.authorizer.clone(),
            inventory_provider: self.inventory_provider.clone(),
            oidc_ctx: self.oidc_ctx.clone(),
        }))
    }
}

/// Authentication middleware
///
/// Whenever an API request is received, the OidcAuthMiddleware will validate the access token and
/// provide the associated user as [`ReqData`](actix_web::web::ReqData) for the subsequent services.
pub struct OidcAuthMiddleware<S> {
    service: Rc<S>,
    settings_provider: SettingsProvider,
    authorizer: Data<Authorizer>,
    inventory_provider: Data<dyn InventoryProvider>,
    oidc_ctx: Data<dyn OidcTokenHandler>,
}

type ResultFuture<O, E> = Pin<Box<dyn Future<Output = Result<O, E>>>>;

impl<S> Service<ServiceRequest> for OidcAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = ResultFuture<Self::Response, Self::Error>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let settings_provider = self.settings_provider.clone();
        let authorizer = self.authorizer.clone();
        let inventory_provider = self.inventory_provider.clone();
        let oidc_ctx = self.oidc_ctx.clone();
        let oidc_cache = req
            .app_data::<Data<Cache>>()
            .expect("Caches must be provided as AppData")
            .clone();

        let parse_match_span = tracing::span!(
            tracing::Level::TRACE,
            "Authorization::<BearerOrInviteCode>::parse"
        );

        let _enter = parse_match_span.enter();
        let auth = match Authorization::<BearerOrInviteCode>::parse(&req) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(
                    "Unable to parse access token or invite code, {}",
                    Report::from_error(e)
                );
                let error = ApiError::unauthorized()
                    .with_message("Unable to parse access token or invite code")
                    .with_www_authenticate(AuthenticationError::InvalidAccessToken);

                let response = req.into_response(error.error_response());
                return Box::pin(ready(Ok(response)));
            }
        };

        enum AccessTokenOrInviteCode {
            AccessToken(AccessToken),
            InviteCode(InviteCode),
        }

        let access_token_or_invite_code = match auth.into_scheme() {
            BearerOrInviteCode::Bearer(bearer) => {
                AccessTokenOrInviteCode::AccessToken(AccessToken::new(bearer.token().to_string()))
            }
            BearerOrInviteCode::InviteCode(invite_code) => {
                AccessTokenOrInviteCode::InviteCode(invite_code)
            }
        };

        Box::pin(
            async move {
                let settings = settings_provider.get();

                match access_token_or_invite_code {
                    AccessTokenOrInviteCode::AccessToken(access_token) => {
                        match access_token::authenticate_user(
                            &settings,
                            &authorizer,
                            inventory_provider.as_ref(),
                            oidc_ctx.as_ref(),
                            oidc_cache.as_ref(),
                            &access_token,
                        )
                        .await
                        {
                            Ok((current_tenant, current_user)) => {
                                // For `POST /v1/rooms/{room_id}/start` a logged-in user may
                                // additionally present an invite code in the JSON body. Peek at the
                                // body here so the authorization middleware's `SubjectCollection`
                                // sees both the `UserId` and the `InviteCode`. This is an interim
                                // measure; the invite-code authentication scheme is scheduled for
                                // removal, at which point this body-peeking hack can be dropped.
                                let body_invite_code =
                                    match peek_room_start_invite_code(&mut req).await {
                                        Ok(code) => code,
                                        Err(err) => {
                                            return Ok(req.into_response(err.error_response()));
                                        }
                                    };

                                req.extensions_mut().insert(current_tenant);
                                req.extensions_mut().insert(current_user.clone());
                                req.extensions_mut().insert(current_user.id);
                                req.extensions_mut()
                                    .insert(build_request_user(current_user));
                                req.extensions_mut().insert(access_token);
                                req.extensions_mut().insert(body_invite_code);
                                if let Some(invite_code) = body_invite_code {
                                    req.extensions_mut().insert(invite_code);
                                }
                                service.call(req).await
                            }
                            Err(err) => Ok(req.into_response(err.error_response())),
                        }
                    }
                    AccessTokenOrInviteCode::InviteCode(current_invite_code) => {
                        req.extensions_mut().insert(current_invite_code);
                        req.extensions_mut().insert(Some(current_invite_code));
                        service.call(req).await
                    }
                }
            }
            .instrument(tracing::trace_span!("OidcAuthMiddleware::async::call")),
        )
    }
}

fn build_request_user(user: User) -> RequestUser {
    RequestUser {
        id: user.id,
        email: user.email,
        title: user.title,
        firstname: user.firstname,
        lastname: user.lastname,
        language: user.language,
        display_name: user.display_name,
        dashboard_theme: user.dashboard_theme,
        conference_theme: user.conference_theme,
        tenant_id: user.tenant_id,
        tariff_id: user.tariff_id,
        tariff_status: user.tariff_status,
        avatar_url: user.avatar_url,
    }
}

/// Peek at the `invite_code` field of a `POST /v1/rooms/{room_id}/start` JSON body without
/// consuming the payload for downstream extractors.
///
/// The `invite_code` for that endpoint is passed in the JSON body rather than in the
/// `Authorization` header, so the authorization middleware would otherwise never see it. This
/// helper buffers the payload, extracts the `invite_code` field, and re-injects the buffered bytes
/// as a fresh payload so the handler's `Json<…>` extractor still works.
async fn peek_room_start_invite_code(
    req: &mut ServiceRequest,
) -> Result<Option<InviteCode>, ApiError> {
    if req.method() != Method::POST {
        return Ok(None);
    }
    // The middleware is wrapped at the `/v1` scope, so route matching has already resolved the
    // pattern by the time we run.
    let Some(pattern) = req.resource_map().match_pattern(req.path()) else {
        return Ok(None);
    };
    if pattern != ROOM_START_PATTERN {
        return Ok(None);
    }

    // Only the JSON handler is registered for this route; other content types are rejected
    // downstream anyway.
    let body = match buffer_payload(req, PEEK_BODY_LIMIT).await {
        Ok(body) => body,
        Err(PayloadError::Overflow) => {
            // This would usually be a 413 - Content Too Large, but we use 400 - Bad Request for the actix json error
            // handler, so we do the same here for consistency.
            return Err(ApiError::bad_request()
                .with_code("payload_overflow")
                .with_message(format!(
                    "Request body exceeds the {PEEK_BODY_LIMIT}-byte limit"
                )));
        }
        Err(err) => {
            tracing::warn!(
                "Failed to buffer body for {}, continuing without invite code: {}",
                ROOM_START_PATTERN,
                Report::from_error(err),
            );
            // Payload is already lost at this point; the downstream extractor will fail cleanly
            // with an empty payload.
            req.set_payload(Payload::from(Bytes::new()));
            return Ok(None);
        }
    };

    #[derive(serde::Deserialize)]
    struct InviteCodePeek {
        invite_code: Option<InviteCode>,
    }

    let invite_code = serde_json::from_slice::<InviteCodePeek>(&body)
        .ok()
        .and_then(|peek| peek.invite_code);

    // Re-inject the buffered body so the endpoint's `Json<…>` extractor can still read it.
    req.set_payload(Payload::from(body));

    Ok(invite_code)
}

/// Read the request payload into memory, aborting with [`PayloadError::Overflow`] as soon as the
/// accumulated body exceeds `limit` bytes.
///
/// The size check runs on each chunk so a client streaming a huge body cannot force us to allocate
/// past the limit before we notice.
async fn buffer_payload(req: &mut ServiceRequest, limit: usize) -> Result<Bytes, PayloadError> {
    let mut payload = req.take_payload();
    let mut buf = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if buf.len().saturating_add(chunk.len()) > limit {
            return Err(PayloadError::Overflow);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.freeze())
}
