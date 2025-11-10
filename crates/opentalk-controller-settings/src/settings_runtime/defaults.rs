// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{collections::BTreeSet, env};

use icu_locid::{LanguageIdentifier, langid};
use opentalk_types_common::{features::ModuleFeatureId, time::TimeZone};

use crate::settings_file;

/// Some settings that apply for the whole installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    /// The user language.
    pub user_language: LanguageIdentifier,

    /// The timezone used by the system and as the users' default.
    pub timezone: TimeZone,

    /// Flag indicating whether the screen share requires an explicit permission.
    pub screen_share_requires_permission: bool,

    /// A list of disabled features.
    pub disabled_features: BTreeSet<ModuleFeatureId>,
}

impl From<settings_file::Defaults> for Defaults {
    fn from(
        settings_file::Defaults {
            user_language,
            timezone,
            screen_share_requires_permission,
            disabled_features,
        }: settings_file::Defaults,
    ) -> Self {
        Self {
            user_language: user_language.unwrap_or_else(default_user_language),
            timezone: timezone.unwrap_or_else(global_timezone),
            screen_share_requires_permission: screen_share_requires_permission.unwrap_or_default(),
            disabled_features,
        }
    }
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            user_language: default_user_language(),
            timezone: TimeZone::default(),
            screen_share_requires_permission: false,
            disabled_features: BTreeSet::default(),
        }
    }
}

pub(crate) fn default_user_language() -> LanguageIdentifier {
    langid!("en-US")
}

pub(crate) fn global_timezone() -> TimeZone {
    // Take timezone from TZ environment variable or OS configuration, fallback to default
    if let Ok(tz) = env::var("TZ").or_else(|_| iana_time_zone::get_timezone())
        && let Ok(tz) = tz.parse()
    {
        tz
    } else {
        TimeZone::default()
    }
}
