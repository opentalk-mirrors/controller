// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use crate::settings_file;

/// Call-in settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallIn {
    /// The call-in telephone number.
    pub tel: String,

    /// Enable mapping of call-in phone number to users with phone numbers known by OpenTalk.
    pub enable_phone_mapping: bool,

    /// Mask unmapped phone numbers to protect privacy.
    pub mask_unmapped_numbers: bool,

    /// The default country code.
    pub default_country_code: phonenumber::country::Id,
}

impl From<settings_file::CallIn> for CallIn {
    fn from(
        settings_file::CallIn {
            tel,
            enable_phone_mapping,
            mask_unmapped_numbers,
            default_country_code,
        }: settings_file::CallIn,
    ) -> Self {
        Self {
            tel,
            enable_phone_mapping,
            mask_unmapped_numbers,
            default_country_code,
        }
    }
}
