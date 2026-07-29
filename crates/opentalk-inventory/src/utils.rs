// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Some helper utilities for interacting with the data storage.

use opentalk_types_common::{
    features::{CALL_IN_MODULE_FEATURE_ID, GUESTS_ALLOWED_MODULE_FEATURE_ID, ModuleFeatureId},
    rooms::GuestAccess,
    tariffs::TariffResource,
    time::Timestamp,
};

use crate::{Inventory, Result, Room, RoomInvite};

/// Why call-in is unavailable for a room
#[derive(Debug)]
pub enum CallInUnavailable {
    /// The room is end-to-end encrypted.
    E2eEnabled,
    /// Guest access is disabled for the room.
    GuestAccessDisabled,
    /// A feature required for call-in is disabled by tariff or configuration.
    FeatureDisabled(ModuleFeatureId),
}

/// Checks whether call-in is available for the given room configuration and
/// tariff, reporting a structured [`CallInUnavailable`] reason on failure.
pub fn check_call_in(
    e2e_encryption: bool,
    guest_access: GuestAccess,
    tariff: &TariffResource,
) -> std::result::Result<(), CallInUnavailable> {
    if e2e_encryption {
        return Err(CallInUnavailable::E2eEnabled);
    }

    if guest_access.is_disabled() {
        return Err(CallInUnavailable::GuestAccessDisabled);
    }

    if !tariff.has_feature_enabled(
        &CALL_IN_MODULE_FEATURE_ID.module,
        &CALL_IN_MODULE_FEATURE_ID.feature,
    ) {
        return Err(CallInUnavailable::FeatureDisabled(
            CALL_IN_MODULE_FEATURE_ID,
        ));
    }

    if !tariff.has_feature_enabled(
        &GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
        &GUESTS_ALLOWED_MODULE_FEATURE_ID.feature,
    ) {
        return Err(CallInUnavailable::FeatureDisabled(
            GUESTS_ALLOWED_MODULE_FEATURE_ID,
        ));
    }

    Ok(())
}

/// Checks if call-in is allowed for a given room and tariff.
pub fn is_call_in_allowed(room: &Room, tariff: &TariffResource) -> bool {
    check_call_in(room.e2e_encryption, room.guest_access, tariff).is_ok()
}

/// Checks if the given `invite` is valid for the given `room` and `tariff`.
pub fn is_invite_valid(invite: &RoomInvite, room: &Room, tariff: &TariffResource) -> bool {
    invite.active
        && invite.room == room.id
        && invite
            .expiration
            .is_none_or(|expiration| expiration > Timestamp::now())
        && is_room_guest_access_allowed(room, tariff)
}

/// Checks if guest access is allowed for a given `room` and `tariff`.
///
/// This only checks if guests are allowed in the room in general. For verifying access with an invite code use
/// [`is_invite_valid`] instead.
pub fn is_room_guest_access_allowed(room: &Room, tariff: &TariffResource) -> bool {
    !room.e2e_encryption
        && !room.guest_access.is_disabled()
        && tariff.has_feature_enabled(
            &GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
            &GUESTS_ALLOWED_MODULE_FEATURE_ID.feature,
        )
}

/// Get a valid invite for a room.
///
/// Returns `Ok(Some(RoomInvite))` when guest access is allowed for the room and a valid invite exists.
/// Returns `Ok(None)` when guest access is not allowed or no valid invite exists.
///
/// # Errors
///
/// Returns an [`Error`](crate::Error) if querying the inventory for the room's active invite fails (database error).
pub async fn get_valid_invite_for_room(
    inventory: &mut dyn Inventory,
    room: &Room,
    tariff: &TariffResource,
) -> Result<Option<RoomInvite>> {
    if is_room_guest_access_allowed(room, tariff) {
        inventory.get_active_invite_for_room(room.id).await
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use opentalk_types_common::{
        features::{CALL_IN_MODULE_FEATURE_ID, GUESTS_ALLOWED_MODULE_FEATURE_ID},
        rooms::{GuestAccess, RoomId, invite_codes::InviteCode},
        tariffs::{TariffId, TariffModuleResource, TariffResource},
        tenants::TenantId,
        time::Timestamp,
        users::UserId,
    };

    use super::{is_invite_valid, is_room_guest_access_allowed};
    use crate::{Room, RoomInvite, utils::is_call_in_allowed};

    #[test]
    fn call_in() {
        let allowed_room = Room {
            id: RoomId::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            password: None,
            waiting_room: true,
            guest_access: GuestAccess::WaitingRoom,
            tenant_id: TenantId::nil(),
            e2e_encryption: false,
        };
        let allowed_tariff = TariffResource {
            id: TariffId::nil(),
            name: "Guest Feature Enabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: {
                // Both features live in the same module (`core`), so the entries
                // must be merged instead of collected directly into a map, where
                // a shared key would cause one feature to overwrite the other.
                let mut modules = BTreeMap::<_, TariffModuleResource>::new();
                let _ = modules
                    .entry(CALL_IN_MODULE_FEATURE_ID.module)
                    .or_default()
                    .features
                    .insert(CALL_IN_MODULE_FEATURE_ID.feature);
                let _ = modules
                    .entry(GUESTS_ALLOWED_MODULE_FEATURE_ID.module)
                    .or_default()
                    .features
                    .insert(GUESTS_ALLOWED_MODULE_FEATURE_ID.feature);
                modules
            },
        };
        assert!(is_call_in_allowed(&allowed_room, &allowed_tariff));

        let encrypted_room = Room {
            e2e_encryption: true,
            ..allowed_room.clone()
        };
        assert!(!is_call_in_allowed(&encrypted_room, &allowed_tariff));

        let call_in_feature_disabled_tariff = TariffResource {
            id: TariffId::nil(),
            name: "Call-In Feature Disabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::from_iter([(
                CALL_IN_MODULE_FEATURE_ID.module,
                TariffModuleResource::default(),
            )]),
        };
        assert!(!is_call_in_allowed(
            &allowed_room,
            &call_in_feature_disabled_tariff
        ));

        let guest_access_disabled_room = Room {
            guest_access: GuestAccess::Disabled,
            ..allowed_room
        };
        assert!(!is_call_in_allowed(
            &guest_access_disabled_room,
            &allowed_tariff
        ));
    }

    #[test]
    fn invite_valid() {
        let allowed_room = Room {
            id: RoomId::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            password: None,
            waiting_room: true,
            guest_access: GuestAccess::WaitingRoom,
            tenant_id: TenantId::nil(),
            e2e_encryption: false,
        };
        let allowed_tariff = TariffResource {
            id: TariffId::nil(),
            name: "Guest Feature Enabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::from_iter([(
                GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
                TariffModuleResource {
                    features: BTreeSet::from([GUESTS_ALLOWED_MODULE_FEATURE_ID.feature]),
                },
            )]),
        };
        let valid_invite = RoomInvite {
            invite_code: InviteCode::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            updated_by: UserId::nil(),
            updated_at: Timestamp::unix_epoch(),
            room: RoomId::nil(),
            active: true,
            expiration: None,
        };
        assert!(is_invite_valid(
            &valid_invite,
            &allowed_room,
            &allowed_tariff
        ));

        let valid_invite = RoomInvite {
            invite_code: InviteCode::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            updated_by: UserId::nil(),
            updated_at: Timestamp::unix_epoch(),
            room: RoomId::nil(),
            active: false,
            expiration: None,
        };
        assert!(!is_invite_valid(
            &valid_invite,
            &allowed_room,
            &allowed_tariff
        ));

        let expired_invite = RoomInvite {
            invite_code: InviteCode::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            updated_by: UserId::nil(),
            updated_at: Timestamp::unix_epoch(),
            room: RoomId::nil(),
            active: true,
            expiration: Some(Timestamp::unix_epoch()),
        };
        assert!(!is_invite_valid(
            &expired_invite,
            &allowed_room,
            &allowed_tariff
        ));
    }

    #[test]
    fn room_guest_access() {
        let allowed_room = Room {
            id: RoomId::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            password: None,
            waiting_room: true,
            guest_access: GuestAccess::WaitingRoom,
            tenant_id: TenantId::nil(),
            e2e_encryption: false,
        };
        let allowed_tariff = TariffResource {
            id: TariffId::nil(),
            name: "Guest Feature Enabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::from_iter([(
                GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
                TariffModuleResource {
                    features: BTreeSet::from([GUESTS_ALLOWED_MODULE_FEATURE_ID.feature]),
                },
            )]),
        };
        assert!(is_room_guest_access_allowed(&allowed_room, &allowed_tariff));

        let encrypted_room = Room {
            id: RoomId::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            password: None,
            waiting_room: true,
            guest_access: GuestAccess::WaitingRoom,
            tenant_id: TenantId::nil(),
            e2e_encryption: true,
        };
        assert!(!is_room_guest_access_allowed(
            &encrypted_room,
            &allowed_tariff
        ));

        let guest_access_disabled_room = Room {
            id: RoomId::nil(),
            id_serial: 0,
            created_by: UserId::nil(),
            created_at: Timestamp::unix_epoch(),
            password: None,
            waiting_room: true,
            guest_access: GuestAccess::Disabled,
            tenant_id: TenantId::nil(),
            e2e_encryption: false,
        };
        assert!(!is_room_guest_access_allowed(
            &guest_access_disabled_room,
            &allowed_tariff
        ));

        let guest_feature_disabled_tariff = TariffResource {
            id: TariffId::nil(),
            name: "Guest Feature Disabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::from_iter([(
                GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
                TariffModuleResource::default(),
            )]),
        };
        assert!(!is_room_guest_access_allowed(
            &allowed_room,
            &guest_feature_disabled_tariff
        ));
    }
}
