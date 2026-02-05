// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Utility to map a phone number to a users display name or convert it to a more readable format

use std::convert::TryFrom;

use opentalk_controller_service::phone_numbers::parse_phone_number;
use opentalk_controller_settings as settings;
use opentalk_inventory::InventoryProvider;
use opentalk_types_common::{tenants::TenantId, users::DisplayName};
use phonenumber::{Mode, PhoneNumber};

/// Try to map the provided phone number to a user
///
/// When the mapping fails or is disabled, the phone number may be formatted to the international phone format.
///
/// Returns the display name for a given SIP display name, e.g. a phone number
pub async fn display_name(
    inventory_provider: &dyn InventoryProvider,
    settings: &settings::CallIn,
    tenant_id: TenantId,
    phone_number_display_name: DisplayName,
) -> DisplayName {
    let Some(parsed_number) = parse_phone_number(
        phone_number_display_name.as_str(),
        settings.default_country_code,
    ) else {
        // Failed to parse, return input
        if settings.mask_unmapped_numbers {
            return DisplayName::from_str_lossy(&mask_str(phone_number_display_name.as_str()));
        }
        return phone_number_display_name;
    };

    if settings.enable_phone_mapping
        && let Some(display_name) =
            try_map_to_user_display_name(inventory_provider, tenant_id, &parsed_number).await
    {
        return display_name;
    }

    let formatted_number = if settings.mask_unmapped_numbers {
        mask_phone_number(&parsed_number)
    } else {
        parsed_number.format().mode(Mode::International).to_string()
    };

    DisplayName::from_str_lossy(&formatted_number)
}

/// Formats a [`PhoneNumber`] in E.164 format, keeping the leading '+' and the first and last 3 digits visible,
/// replacing the rest with '\*\*\*'.
fn mask_phone_number(phone_number: &PhoneNumber) -> String {
    let phone_number = phone_number.format().mode(Mode::E164).to_string();
    mask_str(&phone_number)
}

/// Masks a [`str`], keeping the first 4 and last 3 characters visible and replacing the rest with '\*\*\*'.
/// If the input is shorter than 7 characters, it keeps as many characters as possible at the start
/// and end, replacing the rest with '\*\*\*'.
fn mask_str(phone_number: &str) -> String {
    let len = phone_number.len();
    let start_len = if phone_number.starts_with('+') { 4 } else { 3 }.min(len);
    let end_len = 3.min(len - start_len);

    let start = &phone_number[..start_len];
    let end = &phone_number[len - end_len..];
    format!("{start}***{end}")
}

/// Try to map the provided phone number to a user
///
/// The mapping fails if no user has the provided phone number configured or multiple
/// users have the provided phone number configured.
///
/// Returns [`None`] the phone number is invalid or cannot be parsed
async fn try_map_to_user_display_name(
    inventory_provider: &dyn InventoryProvider,
    tenant_id: TenantId,
    phone_number: &PhoneNumber,
) -> Option<DisplayName> {
    let phone_number_e164 = phone_number
        .format()
        .mode(phonenumber::Mode::E164)
        .to_string();

    let mut inventory = inventory_provider.get_inventory().await.ok()?;

    let result = inventory
        .get_users_by_phone_number(tenant_id, &phone_number_e164)
        .await;

    let users = match result {
        Ok(users) => users,
        Err(err) => {
            log::warn!("Failed to get users by phone number from database {err:?}");
            return None;
        }
    };

    if let Ok([user]) = <[_; 1]>::try_from(users) {
        Some(user.display_name)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use phonenumber::country::Id;
    use pretty_assertions::assert_eq;

    use crate::api::signaling::ws::runner::call_in::mask_phone_number;

    #[test]
    fn mask_phone_number_regular() {
        let phone_number = phonenumber::parse(Some(Id::DE), "1234567890").unwrap();
        let produced = mask_phone_number(&phone_number);
        let expected = "+491***890";
        assert_eq!(expected, produced);
    }

    #[test]
    fn mask_phone_number_short() {
        let phone_number = phonenumber::parse(Some(Id::DE), "12").unwrap();
        let produced = mask_phone_number(&phone_number);
        let expected = "+491***2";
        assert_eq!(expected, produced);
    }

    #[test]
    fn mask_phone_number_too_short() {
        let phone_number = "+491";
        let produced = super::mask_str(phone_number);
        let expected = "+491***";
        assert_eq!(expected, produced);
    }

    #[test]
    fn mask_phone_number_empty() {
        let produced = super::mask_str("");
        let expected = "***";
        assert_eq!(expected, produced);
    }
}
