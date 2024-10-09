// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use serde::Deserialize;

#[derive(Default, Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct Authorization {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synchronize_controllers: Option<bool>,
}
