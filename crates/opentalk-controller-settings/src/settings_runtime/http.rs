// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_service_auth::service::ApiKeys;

use super::{HttpCors, HttpTls};
use crate::settings_file;

pub const DEFAULT_HTTP_PORT: u16 = 11311;

/// The runtime configuration for the HTTP service provided by the OpenTalk controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Http {
    /// The address on which to listen for incoming connections.
    pub addr: Option<String>,

    /// The port on which to listen for incoming connections.
    pub port: u16,

    /// The TLS configuration.
    pub tls: Option<HttpTls>,

    /// The CORS configuration.
    pub cors: HttpCors,

    /// The controllers api keys for service requests
    pub service_api_keys: Option<ApiKeys>,
}

impl From<Option<settings_file::Http>> for Http {
    fn from(value: Option<settings_file::Http>) -> Self {
        value.map(Into::into).unwrap_or_default()
    }
}

impl From<settings_file::Http> for Http {
    fn from(
        settings_file::Http {
            addr,
            port,
            tls,
            cors,
            service_api_keys,
        }: settings_file::Http,
    ) -> Self {
        Self {
            addr,
            port: port.unwrap_or(DEFAULT_HTTP_PORT),
            tls: tls.map(Into::into),
            cors: cors.map(Into::into).unwrap_or_default(),
            service_api_keys,
        }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self {
            addr: None,
            port: DEFAULT_HTTP_PORT,
            tls: None,
            cors: HttpCors::default(),
            service_api_keys: None,
        }
    }
}
