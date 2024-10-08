// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::future::{Ready, ready};

use actix_web::dev::{Service, ServiceRequest, Transform};

use super::AuthorizationService;

/// Transform for authorizing requests to OpenTalk API endpoints.
#[derive(Debug)]
pub struct AuthorizationTransform;

impl<S: Service<ServiceRequest>> Transform<S, ServiceRequest> for AuthorizationTransform {
    type Response = S::Response;
    type Error = S::Error;
    type Transform = AuthorizationService<S>;
    type InitError = S::Error;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthorizationService { service }))
    }
}
