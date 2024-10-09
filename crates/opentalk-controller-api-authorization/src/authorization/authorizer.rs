// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use async_trait::async_trait;

use super::{Access, Admission, Resource, Subject};

/// A trait for implementing authorization queries against the OpenTalk Controller API.
#[async_trait]
pub trait Authorizer {
    /// The error type for failures inside the implementation.
    type Error;

    /// Attempt to authorize for a specific resource as a subject.
    async fn authorize(
        subject: Subject,
        resource: Resource,
        access: Access,
    ) -> Result<Admission, Self::Error>;
}
