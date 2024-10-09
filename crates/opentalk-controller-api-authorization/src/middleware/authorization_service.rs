// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{cell::RefCell, future::Future, pin::Pin, rc::Rc};

use actix_web::{
    HttpResponse,
    dev::{Service, ServiceRequest, ServiceResponse},
};

use crate::authorization::{Admission, AuthorizationTarget, Authorizer};

/// OpenTalk API endpoints authorization middleware.
#[derive(Debug)]
pub struct AuthorizationService<S> {
    service: Rc<RefCell<S>>,
    authorizer: Authorizer,
}

impl<S> AuthorizationService<S> {
    pub(super) fn new(service: Rc<RefCell<S>>, authorizer: Authorizer) -> Self {
        Self {
            service,
            authorizer,
        }
    }
}

type ResultFuture<O, E> = Pin<Box<dyn Future<Output = Result<O, E>>>>;

impl<S: Service<ServiceRequest, Response = ServiceResponse, Error = actix_web::Error> + 'static>
    Service<ServiceRequest> for AuthorizationService<S>
{
    type Response = ServiceResponse;
    type Error = actix_web::Error;
    type Future = ResultFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &self,
        ctx: &mut core::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let authorizer = self.authorizer.clone();
        let service = self.service.clone();
        Box::pin(async move {
            let target = AuthorizationTarget::try_from(&req);
            let admission = if let Ok(target) = target {
                authorizer.authorize(target).await
            } else {
                log::error!("Could not parse path for {:?}, denying access", req.path());
                Ok(Admission::Denied)
            };

            match admission {
                Ok(Admission::Allowed) => service.call(req).await,
                Ok(Admission::Denied) => Ok(req.into_response(HttpResponse::Forbidden().finish())),
                Err(e) => {
                    log::error!("Attempt to request authorization failed: {e:?}");
                    Ok(req.into_response(HttpResponse::InternalServerError().finish()))
                }
            }
        })
    }
}
