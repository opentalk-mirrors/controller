// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides OpenID Connect stuff.

use claims::OpenTalkAdditionalClaims;
use provider::ProviderClient;

mod claims;
mod http;
mod introspect_info;
mod jwt;
mod oidc_context;
mod oidc_token_handler;
mod open_id_connect_user_info;
mod provider;
mod realm_roles;

pub use claims::OnlyExpiryClaim;
use claims::ServiceClaims;
use introspect_info::IntrospectInfo;
pub use jwt::{VerifyError, decode_token};
use oidc_context::OidcContext;
pub use oidc_token_handler::{OidcTokenHandler, build_oidc_token_handler};
pub use open_id_connect_user_info::OpenIdConnectUserInfo;
pub use realm_roles::RealmRoles;
