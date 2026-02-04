// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use core::{
    future::{Future, Ready, ready},
    pin::Pin,
    task::{Context, Poll},
};
use std::rc::Rc;

use actix_http::header::Header;
use actix_web::{
    ResponseError,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use openidconnect::AccessToken;
use opentalk_controller_service_facade::StartRoomError;
use opentalk_controller_settings::SettingsProvider;
use opentalk_roomserver_types::room_parameters::AssetStorageConfig;
use opentalk_types_api_v1::error::{ApiError, AuthenticationError};
use snafu::Report;
use tracing::Instrument;

/// Middleware factory for [`RoomserverAuthMiddleware`]
pub struct RoomserverAuth {
    settings: SettingsProvider,
}

impl RoomserverAuth {
    pub fn new(settings: SettingsProvider) -> Self {
        Self { settings }
    }
}

impl<S> Transform<S, ServiceRequest> for RoomserverAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Transform = RoomserverAuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RoomserverAuthMiddleware {
            service: Rc::new(service),
            settings: self.settings.clone(),
        }))
    }
}

/// Middleware which extracts and verifies an access-token from the request
pub struct RoomserverAuthMiddleware<S> {
    service: Rc<S>,

    settings: SettingsProvider,
}

type ResultFuture<O, E> = Pin<Box<dyn Future<Output = Result<O, E>>>>;

impl<S> Service<ServiceRequest> for RoomserverAuthMiddleware<S>
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

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let Some(secret) = self.get_roomserver_secret() else {
            log::warn!("No RoomServer access token configured, rejecting all requests");
            return Box::pin(async move {
                let api_error: ApiError = StartRoomError::RoomserverSignalingDisabled.into();
                Err(api_error.into())
            });
        };

        let parse_match_span = tracing::trace_span!("Authorization::<Bearer>::parse");

        let _enter = parse_match_span.enter();
        let auth = match Authorization::<Bearer>::parse(&req) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("Unable to parse access token, {}", Report::from_error(e));
                let error = ApiError::unauthorized()
                    .with_message("Unable to parse access token")
                    .with_www_authenticate(AuthenticationError::InvalidAccessToken);

                let response = req.into_response(error.error_response());
                return Box::pin(ready(Ok(response)));
            }
        };

        let access_token = AccessToken::new(auth.into_scheme().token().to_string());

        Box::pin(
            async move {
                if access_token.secret().as_str() == secret.as_str() {
                    service.call(req).await
                } else {
                    log::warn!("Invalid access token");

                    Err(ApiError::unauthorized()
                        .with_www_authenticate(AuthenticationError::InvalidAccessToken)
                        .into())
                }
            }
            .instrument(tracing::trace_span!(
                "RoomserverAuthMiddleware::async::call"
            )),
        )
    }
}

impl<S> RoomserverAuthMiddleware<S> {
    fn get_roomserver_secret(&self) -> Option<String> {
        self.settings.get().roomserver.as_ref().and_then(|r| {
            if let AssetStorageConfig::Controller { secret, .. } = &r.asset_storage {
                Some(secret.clone())
            } else {
                None
            }
        })
    }
}
