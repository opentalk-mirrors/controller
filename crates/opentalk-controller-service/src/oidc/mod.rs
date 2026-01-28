// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Provides OpenID Connect stuff.

mod cache;
mod cache_token;
mod claims;
mod introspect_info;
mod jwt;
mod oidc_context;
mod oidc_token_handler;
mod open_id_connect_user_info;
mod provider;
mod realm_roles;

pub use cache::Cache;
pub use cache_token::{insert_access_token, upsert_access_token_patch_me};
pub use claims::OnlyExpiryClaim;
use claims::{OpenTalkAdditionalClaims, ServiceClaims};
use introspect_info::IntrospectInfo;
pub use jwt::{VerifyError, decode_token};
use oidc_context::OidcContext;
pub use oidc_token_handler::{OidcTokenHandler, build_oidc_token_handler};
pub use open_id_connect_user_info::OpenIdConnectUserInfo;
use provider::ProviderClient;
pub use realm_roles::RealmRoles;
