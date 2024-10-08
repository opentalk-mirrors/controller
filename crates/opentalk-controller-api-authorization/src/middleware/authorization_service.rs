// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use actix_web::dev::{Service, ServiceRequest};

/// OpenTalk API endpoints authorization middleware.
#[derive(Debug)]
pub struct AuthorizationService<S> {
    pub(super) service: S,
}

impl<S: Service<ServiceRequest>> Service<ServiceRequest> for AuthorizationService<S> {
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &self,
        ctx: &mut core::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        self.service.call(req)
    }
}
