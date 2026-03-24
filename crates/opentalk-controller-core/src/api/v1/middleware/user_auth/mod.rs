// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Handles user Authentication in API requests
use core::future::ready;
use std::{
    collections::BTreeSet,
    future::{Future, Ready},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use actix_web::{
    HttpMessage, ResponseError,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
    http::header::Header,
    web::Data,
};
use actix_web_httpauth::headers::authorization::Authorization;
use diesel_async::scoped_futures::ScopedFutureExt as _;
use kustos::prelude::PoliciesBuilder;
use openidconnect::AccessToken;
use opentalk_controller_service::{
    controller_backend::RoomsPoliciesBuilderExt,
    oidc::{Cache, OidcTokenHandler, OpenIdConnectUserInfo},
    phone_numbers::parse_phone_number,
};
use opentalk_controller_service_facade::RequestUser;
use opentalk_controller_settings::{
    Settings, SettingsProvider, TariffAssignment, TariffStatusMapping, TenantAssignment,
};
use opentalk_controller_utils::CaptureApiError;
use opentalk_inventory::{
    Inventory, InventoryProvider, NewUser, Tariff, Tenant, UpsertOutcome, User, transaction,
};
use opentalk_types_api_v1::error::{ApiError, AuthenticationError};
use opentalk_types_common::{
    events::EventId,
    rooms::{RoomId, invite_codes::InviteCode},
    tariffs::TariffStatus,
    tenants::TenantId,
    users::{DisplayName, GroupId, GroupName, UserTitle},
};
use snafu::Report;
use tracing_futures::Instrument;
use uuid::Uuid;

use crate::api::v1::{
    events::EventPoliciesBuilderExt,
    middleware::user_auth::bearer_or_invite_code::BearerOrInviteCode,
};

mod bearer_or_invite_code;

/// Middleware factory
///
/// Transforms into [`OidcAuthMiddleware`]
pub struct OidcAuth {
    pub settings_provider: SettingsProvider,
    pub inventory_provider: Data<dyn InventoryProvider>,
    pub authz: Data<kustos::Authz>,
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
            authz: self.authz.clone(),
            inventory_provider: self.inventory_provider.clone(),
            oidc_ctx: self.oidc_ctx.clone(),
        }))
    }
}

/// Authentication middleware
///
/// Whenever an API request is received, the OidcAuthMiddleware will validate the access
/// token and provide the associated user as [`ReqData`](actix_web::web::ReqData) for the subsequent services.
pub struct OidcAuthMiddleware<S> {
    service: Rc<S>,
    settings_provider: SettingsProvider,
    authz: Data<kustos::Authz>,
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

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let settings_provider = self.settings_provider.clone();
        let authz = self.authz.clone();
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
                log::warn!(
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
                    AccessTokenOrInviteCode::AccessToken(access_token) => match check_access_token(
                        &settings,
                        &authz,
                        inventory_provider.as_ref(),
                        oidc_ctx.as_ref(),
                        oidc_cache.as_ref(),
                        &access_token,
                    )
                    .await
                    {
                        Ok((current_tenant, current_user)) => {
                            req.extensions_mut()
                                .insert(kustos::actix_web::User::from(Uuid::from(current_user.id)));
                            req.extensions_mut().insert(current_tenant);
                            req.extensions_mut().insert(current_user.clone());
                            req.extensions_mut()
                                .insert(build_request_user(current_user));
                            req.extensions_mut().insert(access_token);
                            req.extensions_mut().insert(None::<InviteCode>);
                            service.call(req).await
                        }
                        Err(err) => Ok(req.into_response(err.error_response())),
                    },
                    AccessTokenOrInviteCode::InviteCode(current_invite_code) => {
                        req.extensions_mut()
                            .insert(kustos::actix_web::Invite::from(Uuid::from(
                                current_invite_code,
                            )));
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

#[tracing::instrument(skip_all)]
pub async fn check_access_token(
    settings: &Settings,
    authz: &kustos::Authz,
    inventory_provider: &dyn InventoryProvider,
    oidc_ctx: &dyn OidcTokenHandler,
    oidc_cache: &Cache,
    access_token: &AccessToken,
) -> Result<(Tenant, User), CaptureApiError> {
    // Check if access token has been cached already
    if let Some(result) = oidc_cache.get_access_token(access_token).await {
        return result;
    }

    // Verifiy access token which is not cached yet
    // On verification error, cache the error and return early
    let verification_result = oidc_ctx.verify_access_token(access_token).await;
    match verification_result {
        Ok(verification_info) => {
            let inner_result = check_access_token_inner(
                settings,
                authz,
                inventory_provider,
                oidc_ctx,
                access_token,
            )
            .await;
            oidc_cache
                .insert_access_token(access_token, inner_result.clone(), verification_info.exp)
                .await
                .is_err()
                .then(|| log::warn!("Failed to cache user data error for access token"));

            inner_result
        }
        Err(error) => {
            oidc_cache
                .insert_access_token(access_token, Err(error.clone()), None)
                .await
                .is_err()
                .then(|| log::warn!("Failed to cache verification error for access token"));
            return Err(error);
        }
    }
}

/// Fetches all associated user data of the access token
async fn check_access_token_inner(
    settings: &Settings,
    authz: &kustos::Authz,
    inventory_provider: &dyn InventoryProvider,
    oidc_ctx: &dyn OidcTokenHandler,
    access_token: &AccessToken,
) -> Result<(Tenant, User), CaptureApiError> {
    let info = oidc_ctx.user_info(access_token.clone()).await?;

    let mut inventory = inventory_provider.get_inventory().await?;

    // Get tariff depending on the configured assignment
    let (tariff, tariff_status) = match &settings.tariffs.assignment {
        TariffAssignment::Static { static_tariff_name } => (
            inventory.get_tariff_by_name(static_tariff_name).await?,
            TariffStatus::Default,
        ),
        TariffAssignment::ByExternalTariffId { status_mapping } => {
            let external_tariff_id = info.tariff_id.clone().ok_or_else(|| {
                ApiError::bad_request()
                    .with_code("invalid_claims")
                    .with_message("tariff_id missing in id_token claims")
            })?;

            let tariff = inventory
                .get_tariff_by_external_tariff_id(external_tariff_id.into())
                .await?
                .ok_or_else(|| {
                    ApiError::internal()
                        .with_code("invalid_tariff_id")
                        .with_message("JWT contained unknown tariff_id")
                })?;

            if let Some(mapping) = status_mapping.as_ref() {
                let status_name = info.tariff_status.clone().ok_or_else(|| {
                    ApiError::bad_request()
                        .with_code("invalid_claims")
                        .with_message("tariff_status missing in id_token claims")
                })?;

                let status = map_tariff_status_name(mapping, &status_name);

                let tariff = if matches!(status, TariffStatus::Downgraded) {
                    inventory
                        .get_tariff_by_name(&mapping.downgraded_tariff_name)
                        .await
                        .map_err(|_| {
                            ApiError::internal()
                                .with_code("invalid_configuration")
                                .with_message("Unable to load downgraded tariff")
                        })?
                } else {
                    tariff
                };
                (tariff, status)
            } else {
                (tariff, TariffStatus::Default)
            }
        }
    };

    // Get the tenant_id depending on the configured assignment
    let tenant_id = match &settings.tenants.assignment {
        TenantAssignment::Static { static_tenant_id } => static_tenant_id.clone(),
        TenantAssignment::ByExternalTenantId { .. } => info.tenant_id.clone().ok_or_else(|| {
            log::error!("Invalid access token, missing tenant_id");
            ApiError::unauthorized().with_www_authenticate(AuthenticationError::InvalidAccessToken)
        })?,
    };
    let tenant = inventory
        .get_or_create_tenant_by_oidc_id(&tenant_id.into())
        .await?;

    let groups: Vec<(TenantId, GroupName)> = info
        .groups
        .iter()
        .map(|group| (tenant.id, GroupName::from(group.clone())))
        .collect();
    let groups = inventory
        .get_or_create_groups_by_name(&groups)
        .await?
        .into_iter()
        .map(|g| g.id)
        .collect::<Vec<GroupId>>();

    let login_result = {
        let tenant = tenant.clone();
        transaction(inventory.as_mut(), |inventory| {
            async move {
                create_or_update_user(
                    inventory,
                    tenant,
                    info,
                    settings,
                    &groups,
                    tariff,
                    tariff_status,
                )
                .await
            }
            .scope_boxed()
        })
        .await?
    };

    let user = update_core_user_permissions(authz, login_result).await?;

    Ok((tenant, user))
}

#[allow(clippy::too_many_arguments)]
async fn create_or_update_user(
    inventory: &mut dyn Inventory,
    tenant: Tenant,
    info: OpenIdConnectUserInfo,
    settings: &Settings,
    groups: &[GroupId],
    tariff: Tariff,
    tariff_status: TariffStatus,
) -> Result<LoginResult, CaptureApiError> {
    let display_name = build_info_display_name(&info);
    let enforce_display_name = settings.endpoints.disallow_custom_display_name;

    let phone_number = if let Some((call_in, phone_number)) =
        settings.call_in.as_ref().zip(info.phone_number.as_deref())
    {
        parse_phone_number(phone_number, call_in.default_country_code)
            .map(|p| p.format().mode(phonenumber::Mode::E164).to_string())
    } else {
        None
    };

    let outcome = inventory
        .create_or_update_user_by_oidc_sub(
            NewUser {
                oidc_sub: info.sub,
                email: info.email,
                title: UserTitle::new(),
                display_name,
                firstname: info.firstname,
                lastname: info.lastname,
                avatar_url: info.avatar_url,
                language: info.locale,
                phone: phone_number,
                tenant_id: tenant.id,
                tariff_id: tariff.id,
                tariff_status,
                timezone: info.timezone,
            },
            enforce_display_name,
        )
        .await?;

    // Set last_authenticated_at field to now for each new access token
    inventory
        .set_last_authenticated_at_to_now(outcome.clone().into_inner().id)
        .await?;

    match outcome {
        UpsertOutcome::Inserted(user) => {
            let groups = inventory.add_user_to_groups(user.id, groups).await?;

            let event_and_room_ids = inventory
                .migrate_event_email_invites_to_user_invites(user.clone())
                .await?;

            Ok(LoginResult::UserCreated {
                user,
                groups,
                event_and_room_ids,
            })
        }
        UpsertOutcome::Updated(user) => {
            let groups_added_to = inventory.add_user_to_groups(user.id, groups).await?;
            let groups_removed_from = inventory
                .remove_user_from_all_groups_except(user.id, groups)
                .await?;

            Ok(LoginResult::UserUpdated {
                user,
                groups_added_to,
                groups_removed_from,
            })
        }
    }
}

fn map_tariff_status_name(mapping: &TariffStatusMapping, name: &String) -> TariffStatus {
    if mapping.default.contains(name) {
        TariffStatus::Default
    } else if mapping.paid.contains(name) {
        TariffStatus::Paid
    } else if mapping.downgraded.contains(name) {
        TariffStatus::Downgraded
    } else {
        log::error!("Invalid tariff status value found: \"{name}\"");
        TariffStatus::Default
    }
}

enum LoginResult {
    UserCreated {
        user: User,
        groups: BTreeSet<GroupId>,
        event_and_room_ids: Vec<(EventId, RoomId)>,
    },
    UserUpdated {
        user: User,
        groups_added_to: BTreeSet<GroupId>,
        groups_removed_from: BTreeSet<GroupId>,
    },
}

async fn update_core_user_permissions(
    authz: &kustos::Authz,
    db_result: LoginResult,
) -> Result<User, CaptureApiError> {
    match db_result {
        LoginResult::UserUpdated {
            user,
            groups_added_to,
            groups_removed_from,
        } => {
            for group_id in groups_added_to {
                authz.add_user_to_group(user.id, group_id).await?;
            }

            for group_id in groups_removed_from {
                authz.remove_user_from_group(user.id, group_id).await?;
            }

            Ok(user)
        }
        LoginResult::UserCreated {
            user,
            groups,
            event_and_room_ids,
        } => {
            authz.add_user_to_role(user.id, "user").await?;

            for group_id in groups {
                authz.add_user_to_group(user.id, group_id).await?;
            }

            // Migrate email invites to user invites
            // Add permissions for user to events that the email was invited to
            if event_and_room_ids.is_empty() {
                return Ok(user);
            }

            let mut policies = PoliciesBuilder::new().grant_user_access(user.id);

            for (event_id, room_id) in event_and_room_ids {
                policies = policies
                    .event_read_access(event_id)
                    .room_read_access(room_id)
                    .event_invite_invitee_access(event_id);
            }

            authz.add_policies(policies.finish()).await?;

            Ok(user)
        }
    }
}

fn build_info_display_name(info: &OpenIdConnectUserInfo) -> DisplayName {
    DisplayName::from_str_lossy(
        &info
            .display_name
            .clone()
            .unwrap_or_else(|| format!("{} {}", &info.firstname, &info.lastname)),
    )
}
