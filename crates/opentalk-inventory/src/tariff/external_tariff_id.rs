// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

/// The external tariff id that can be mapped to a tariff.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::AsRef,
    derive_more::Display,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ExternalTariffId(String);

impl From<opentalk_db_storage::tariffs::ExternalTariffId> for ExternalTariffId {
    fn from(value: opentalk_db_storage::tariffs::ExternalTariffId) -> Self {
        Self(value.into())
    }
}

impl From<ExternalTariffId> for opentalk_db_storage::tariffs::ExternalTariffId {
    fn from(ExternalTariffId(value): ExternalTariffId) -> Self {
        Self::from(value)
    }
}
