// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use super::{
    Authorization, Avatar, CallIn, Database, Defaults, Endpoints, Etcd, Frontend, Http, Logging,
    Metrics, MinIO, Monitoring, Oidc, OperatorInformation, RabbitMq, Redis, SharedFolder, Tariffs,
    Tenants, UserSearchBackend, oidc_and_user_search_builder::OidcAndUserSearchBuilder,
};
use crate::{
    Result, SettingsError, SettingsRaw, settings_file::UsersFindBehavior,
    settings_runtime::RoomServer,
};

/// The settings used for the OpenTalk controller at runtime
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// The frontend information.
    pub frontend: Frontend,

    /// The OIDC configuration for OpenTalk.
    pub oidc: Oidc,

    /// The user search backend.
    pub user_search_backend: Option<UserSearchBackend>,

    /// The user search behavior.
    pub users_find_behavior: UsersFindBehavior,

    /// The HTTP service settings.
    pub http: Http,

    /// The database connection settings.
    pub database: Database,

    /// The redis connection settings.
    pub redis: Option<Redis>,

    /// The RabbitMQ connection settings.
    pub rabbit_mq: Option<RabbitMq>,

    /// The Authorization settings.
    pub authorization: Authorization,

    /// The logging settings.
    pub logging: Logging,

    /// The avatar settings.
    pub avatar: Avatar,

    /// The metrics settings.
    pub metrics: Metrics,

    /// The etcd settings.
    pub etcd: Option<Etcd>,

    /// The SharedFolder settings.
    pub shared_folder: Option<SharedFolder>,

    /// The endpoint settings.
    pub endpoints: Endpoints,

    /// The minio settings.
    pub minio: MinIO,

    /// The monitoring settings.
    pub monitoring: Option<Monitoring>,

    /// The call-in settings.
    pub call_in: Option<CallIn>,

    /// The tenant configuration.
    pub tenants: Tenants,

    /// The tariff configuration.
    pub tariffs: Tariffs,

    /// The defaults configuration.
    pub defaults: Defaults,

    /// Information about the operator.
    pub operator_information: Option<OperatorInformation>,

    /// The roomserver configuration
    pub roomserver: RoomServer,
}

impl Settings {
    pub(crate) fn try_reload_from(&mut self, new_raw: SettingsRaw) -> Result<()> {
        let new = Settings::try_from(new_raw)?;

        // reload metrics
        self.metrics = new.metrics;

        // reload avatar
        self.avatar = new.avatar;

        // reload call in
        self.call_in = new.call_in;

        Ok(())
    }
}

impl TryFrom<SettingsRaw> for Settings {
    type Error = SettingsError;

    fn try_from(raw: SettingsRaw) -> Result<Self, Self::Error> {
        let OidcAndUserSearchBuilder {
            oidc,
            user_search_backend,
            users_find_behavior,
        } = OidcAndUserSearchBuilder::load_from_settings_raw(&raw)?;

        let frontend = raw.frontend.clone().into();
        let http = Http::from(raw.http.clone());
        let database = raw.database.clone().into();
        let redis = raw.redis.clone().map(Into::into);
        let rabbit_mq = raw.rabbit_mq.clone().map(Into::into);
        let authorization =
            Authorization::from_settings_file(raw.authorization.clone(), rabbit_mq.is_some());
        let logging = raw.logging.clone().map(Into::into).unwrap_or_default();
        let avatar = raw.avatar.clone().map(Into::into).unwrap_or_default();
        let metrics = raw.metrics.clone().map(Into::into).unwrap_or_default();
        let etcd = raw.etcd.clone().map(Into::into);
        let shared_folder = raw.shared_folder.clone().map(Into::into);
        let endpoints = raw.endpoints.clone().map(Into::into).unwrap_or_default();
        let minio = raw.minio.clone().into();
        let monitoring = raw.monitoring.clone().map(Into::into);
        let call_in = raw.call_in.clone().map(Into::into);
        let tenants = raw.tenants.clone().map(Into::into).unwrap_or_default();
        let tariffs = raw.tariffs.clone().map(Into::into).unwrap_or_default();
        let defaults = raw.defaults.clone().map(Into::into).unwrap_or_default();
        let operator_information = raw.operator_information.clone().map(Into::into);
        let roomserver = raw.roomserver.clone().into();

        if http.service_api_keys.is_none() {
            return Err(SettingsError::HttpServiceApiKeysMissing);
        }

        Ok(Settings {
            frontend,
            oidc,
            user_search_backend,
            users_find_behavior,
            http,
            database,
            redis,
            rabbit_mq,
            authorization,
            logging,
            avatar,
            metrics,
            etcd,
            shared_folder,
            endpoints,
            minio,
            monitoring,
            call_in,
            tenants,
            tariffs,
            defaults,
            operator_information,
            roomserver,
        })
    }
}

#[cfg(test)]
pub(crate) fn minimal_example() -> Settings {
    use std::collections::BTreeSet;

    use openidconnect::{ClientId, ClientSecret};
    use opentalk_roomserver_types::{
        module_settings::ModuleSettings, rate_limit::RateLimitSettings,
    };
    use opentalk_service_auth::{ApiKey, service::ApiKeys};
    use opentalk_types_common::time::TimeZone;
    use url::Url;

    use super::OidcController;
    use crate::{
        DEFAULT_LIBRAVATAR_URL, DEFAULT_STATIC_TARIFF_NAME, DEFAULT_STATIC_TENANT_ID, Frontend,
        OidcFrontend, TariffAssignment, TenantAssignment,
        settings_runtime::{
            HttpCors,
            database::DEFAULT_DATABASE_MAX_CONNECTIONS,
            defaults::default_user_language,
            http::{DEFAULT_HTTP_PORT, DEFAULT_UPLOAD_SIZE_LIMIT},
        },
    };

    Settings {
        frontend: Frontend {
            base_url: Url::parse("https://example.com").unwrap(),
        },
        oidc: Oidc {
            controller: OidcController {
                authority: "http://localhost:8080/realms/opentalk"
                    .parse()
                    .expect("must be a valid url"),
                client_id: ClientId::new("Controller".to_string()),
                client_secret: ClientSecret::new("mysecret".to_string()),
            },
            frontend: OidcFrontend {
                authority: "http://localhost:8080/realms/opentalk"
                    .parse()
                    .expect("must be a valid url"),
                client_id: ClientId::new("Webapp".to_string()),
            },
        },
        user_search_backend: None,
        users_find_behavior: UsersFindBehavior::Disabled,
        http: Http {
            addr: None,
            port: DEFAULT_HTTP_PORT,
            tls: None,
            cors: HttpCors::default(),
            service_api_keys: Some(ApiKeys::new(vec![ApiKey::new("controller", "secret")])),
            upload_size_limit: DEFAULT_UPLOAD_SIZE_LIMIT,
        },
        database: Database {
            url: "postgres://postgres:password123@localhost:5432/opentalk".to_string(),
            max_connections: DEFAULT_DATABASE_MAX_CONNECTIONS,
        },
        redis: None,
        rabbit_mq: None,
        authorization: Authorization {
            synchronize_controllers: false,
        },
        logging: Logging {
            default_directives: None,
            otlp_tracing: None,
        },
        avatar: Avatar {
            libravatar_url: DEFAULT_LIBRAVATAR_URL.to_string(),
        },
        metrics: Metrics { allowlist: vec![] },
        etcd: None,
        shared_folder: None,
        endpoints: Endpoints {
            event_invite_external_email_address: false,
            disallow_custom_display_name: false,
            disable_openapi: false,
        },
        minio: MinIO {
            uri: "http://localhost:9555"
                .parse()
                .expect("must be a valid url"),
            bucket: "controller".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            region: None,
            force_path_style: None,
        },
        monitoring: None,
        call_in: None,
        tenants: Tenants {
            assignment: TenantAssignment::Static {
                static_tenant_id: DEFAULT_STATIC_TENANT_ID.to_string(),
            },
        },
        tariffs: Tariffs {
            assignment: TariffAssignment::Static {
                static_tariff_name: DEFAULT_STATIC_TARIFF_NAME.to_string(),
            },
        },
        defaults: Defaults {
            user_language: default_user_language(),
            timezone: TimeZone::default(),
            disabled_features: BTreeSet::new(),
        },
        operator_information: None,
        roomserver: RoomServer {
            url: "http://localhost:11333"
                .parse()
                .expect("must be a valid url"),
            api_key: ApiKey::new("roomserver", "secret"),
            modules: ModuleSettings::new(),
            websocket_rate_limit: Some(RateLimitSettings {
                tokens_per_second: 10,
                token_bucket_size: 30,
            }),
        },
    }
}
