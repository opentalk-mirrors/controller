// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use async_trait::async_trait;
use opentalk_controller_api_authorization::authorization::{
    Admission, AuthorizationChange, AuthorizationChangeError, AuthorizationError,
    AuthorizationTarget, AuthorizerBackend,
};

use crate::data::AuthorizationData;

/// The [`AuthorizerBackend`] for OpenTalk
#[derive(Default, Debug, PartialEq, Eq)]
pub struct OpenTalkAuthorizerBackend {
    data: AuthorizationData,
}

#[async_trait]
impl AuthorizerBackend for OpenTalkAuthorizerBackend {
    async fn authorize(
        &self,
        authorization_target: AuthorizationTarget,
    ) -> Result<Admission, AuthorizationError> {
        Ok(self.data.authorize(authorization_target))
    }

    async fn apply_changes(
        &mut self,
        changeset: &[AuthorizationChange],
    ) -> Result<(), AuthorizationChangeError> {
        self.data.apply_changes(changeset);
        Ok(())
    }
}

impl OpenTalkAuthorizerBackend {
    /// Create a new [`OpenTalkAuthorizerBackend`].
    pub fn new() -> Self {
        Self {
            data: AuthorizationData::default(),
        }
    }

    /// Build a new [`OpenTalkAuthorizerBackend`] and pre-fill it with the
    /// changes from a set of [`AuthorizationChange`] entries.
    pub fn new_from_changeset(changeset: &[AuthorizationChange]) -> Self {
        Self {
            data: AuthorizationData::build_from_changeset(changeset),
        }
    }
}
