// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use super::ReportsTypst;
use crate::settings_file;

/// The runtime configuration for generating reports.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Reports {
    /// The typst-specific report generation configuration.
    pub typst: ReportsTypst,
}

impl From<settings_file::Reports> for Reports {
    fn from(settings_file::Reports { typst }: settings_file::Reports) -> Self {
        Self {
            typst: typst.map(|typst| typst.into()).unwrap_or_default(),
        }
    }
}
